use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use snek_shared::*;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::session;

type WsTx = SplitSink<WebSocketStream<TcpStream>, Message>;
type WsRx = SplitStream<WebSocketStream<TcpStream>>;

struct WaitingPlayer {
    info: PlayerInfo,
    /// Channel to send outbound messages TO this player's WebSocket.
    tx: mpsc::Sender<String>,
    /// Signal: when matched, receives the peer's outbound tx so this handler
    /// knows where to forward incoming messages.
    matched: oneshot::Sender<mpsc::Sender<String>>,
}

pub struct Lobby {
    queue: Mutex<Vec<WaitingPlayer>>,
    color_counter: AtomicUsize,
    pub active_connections: AtomicUsize,
}

const COLORS: [SnakeColor; 6] = [
    SnakeColor::Blue,
    SnakeColor::Magenta,
    SnakeColor::Cyan,
    SnakeColor::Yellow,
    SnakeColor::Green,
    SnakeColor::Red,
];

impl Lobby {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            color_counter: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
        }
    }

    fn next_color(&self) -> SnakeColor {
        let idx = self.color_counter.fetch_add(1, Ordering::Relaxed);
        COLORS[idx % COLORS.len()]
    }

    pub async fn handle_client(&self, mut ws_tx: WsTx, mut ws_rx: WsRx, peer_addr: SocketAddr) {
        let (out_tx, mut out_rx) = mpsc::channel::<String>(RELAY_CHANNEL_BOUND);

        // Writer task: forwards channel messages to WebSocket
        let write_task = tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if let Err(e) = ws_tx.send(Message::Text(msg.into())).await {
                    eprintln!("[relay] writer task: WebSocket send error: {e}");
                    break;
                }
            }
            ws_tx
        });

        let mut player_info: Option<PlayerInfo> = None;

        // Main lobby read loop
        while let Some(msg_result) = ws_rx.next().await {
            let msg = match msg_result {
                Ok(Message::Text(t)) => t.to_string(),
                Ok(Message::Close(_)) => {
                    eprintln!("[lobby] client {peer_addr} sent Close frame");
                    break;
                }
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("[lobby] WebSocket read error from {peer_addr}: {e}");
                    break;
                }
            };

            let Ok(relay_msg) = serde_json::from_str::<ClientRelayMessage>(&msg) else {
                let _ = out_tx.try_send(
                    serde_json::to_string(&ServerRelayMessage::Error {
                        message: "Invalid message".into(),
                    })
                    .unwrap(),
                );
                continue;
            };

            match relay_msg {
                ClientRelayMessage::Register { name } => {
                    let trimmed = name.trim().to_string();
                    if trimmed.is_empty()
                        || trimmed.len() > MAX_NAME_LENGTH
                        || !trimmed.chars().all(|c| c.is_alphanumeric())
                    {
                        let _ = out_tx.try_send(
                            serde_json::to_string(&ServerRelayMessage::Error {
                                message: "Name must be 1-16 alphanumeric characters".into(),
                            })
                            .unwrap(),
                        );
                        continue;
                    }
                    let info = PlayerInfo {
                        id: Uuid::new_v4(),
                        name: trimmed,
                        color: self.next_color(),
                    };
                    let _ = out_tx.try_send(
                        serde_json::to_string(&ServerRelayMessage::Registered {
                            player: info.clone(),
                        })
                        .unwrap(),
                    );
                    println!("Player '{}' registered from {peer_addr}", info.name);
                    player_info = Some(info);
                }
                ClientRelayMessage::SearchMatch => {
                    let Some(ref info) = player_info else {
                        let _ = out_tx.try_send(
                            serde_json::to_string(&ServerRelayMessage::Error {
                                message: "Must register first".into(),
                            })
                            .unwrap(),
                        );
                        continue;
                    };

                    let matched = {
                        let mut queue = self.queue.lock().unwrap();
                        if queue.is_empty() {
                            None
                        } else {
                            Some(queue.remove(0))
                        }
                    };

                    if let Some(opponent) = matched {
                        // Found a match — pair them up
                        let game_id = Uuid::new_v4();

                        let host_is_current = game_id.as_bytes()[0] & 1 == 0;
                        let (current_role, opponent_role) = if host_is_current {
                            (PeerRole::Host, PeerRole::Guest)
                        } else {
                            (PeerRole::Guest, PeerRole::Host)
                        };

                        // Send MatchFound to both players with fixed board size
                        let _ = out_tx.try_send(
                            serde_json::to_string(&ServerRelayMessage::MatchFound {
                                game_id,
                                role: current_role,
                                board_width: MP_BOARD_WIDTH,
                                board_height: MP_BOARD_HEIGHT,
                                opponent: opponent.info.clone(),
                                you: info.clone(),
                            })
                            .unwrap(),
                        );
                        let _ = opponent.tx.try_send(
                            serde_json::to_string(&ServerRelayMessage::MatchFound {
                                game_id,
                                role: opponent_role,
                                board_width: MP_BOARD_WIDTH,
                                board_height: MP_BOARD_HEIGHT,
                                opponent: info.clone(),
                                you: opponent.info.clone(),
                            })
                            .unwrap(),
                        );

                        // Signal the waiting player's handler to enter relay mode,
                        // giving them OUR out_tx so they can forward our messages back to us.
                        let _ = opponent.matched.send(out_tx.clone());

                        println!(
                            "Match found: '{}' vs '{}' (game {game_id})",
                            info.name, opponent.info.name
                        );

                        // Enter relay mode: read from our ws_rx, forward to opponent
                        session::relay_session(&opponent.tx, &mut ws_rx).await;
                        break;
                    } else {
                        // No match yet — add to queue and wait for signal
                        let (match_tx, mut match_rx) = oneshot::channel();
                        {
                            let mut queue = self.queue.lock().unwrap();
                            queue.push(WaitingPlayer {
                                info: info.clone(),
                                tx: out_tx.clone(),
                                matched: match_tx,
                            });
                        }
                        let _ = out_tx.try_send(
                            serde_json::to_string(&ServerRelayMessage::SearchingMatch).unwrap(),
                        );

                        // Wait for either a match signal or a client message (CancelSearch)
                        let peer_tx = loop {
                            tokio::select! {
                                result = &mut match_rx => {
                                    break result.ok();
                                }
                                msg_opt = ws_rx.next() => {
                                    match msg_opt {
                                        Some(Ok(Message::Text(t))) => {
                                            let text = t.to_string();
                                            if let Ok(ClientRelayMessage::CancelSearch) =
                                                serde_json::from_str::<ClientRelayMessage>(&text)
                                            {
                                                if let Some(ref info) = player_info {
                                                    let mut queue = self.queue.lock().unwrap();
                                                    queue.retain(|w| w.info.id != info.id);
                                                }
                                                let _ = out_tx.try_send(
                                                    serde_json::to_string(
                                                        &ServerRelayMessage::SearchCancelled,
                                                    )
                                                    .unwrap(),
                                                );
                                                break None;
                                            }
                                            if let Ok(ClientRelayMessage::Disconnect) =
                                                serde_json::from_str::<ClientRelayMessage>(&text)
                                            {
                                                break None;
                                            }
                                        }
                                        _ => break None, // disconnected
                                    }
                                }
                            }
                        };

                        if let Some(peer_tx) = peer_tx {
                            // Matched! Enter relay mode
                            session::relay_session(&peer_tx, &mut ws_rx).await;
                        }
                        break;
                    }
                }
                ClientRelayMessage::CancelSearch => {
                    if let Some(ref info) = player_info {
                        let mut queue = self.queue.lock().unwrap();
                        queue.retain(|w| w.info.id != info.id);
                    }
                    let _ = out_tx.try_send(
                        serde_json::to_string(&ServerRelayMessage::SearchCancelled).unwrap(),
                    );
                }
                ClientRelayMessage::Disconnect => {
                    break;
                }
            }
        }

        // Cleanup: remove from queue if still there
        if let Some(ref info) = player_info {
            let mut queue = self.queue.lock().unwrap();
            queue.retain(|w| w.info.id != info.id);
        }

        drop(out_tx);
        let _ = write_task.await;
        if let Some(ref info) = player_info {
            println!("Player '{}' ({peer_addr}) disconnected", info.name);
        } else {
            println!("Client {peer_addr} disconnected");
        }
    }
}
