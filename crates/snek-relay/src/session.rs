use std::time::Instant;

use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use snek_shared::{PeerMessage, RATE_LIMIT_BURST, RATE_LIMIT_REFILL_MS, ServerRelayMessage};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

type WsRx = SplitStream<WebSocketStream<TcpStream>>;

struct RateLimit {
    tokens: u32,
    max_tokens: u32,
    last_refill: Instant,
    refill_rate_ms: u64,
}

impl RateLimit {
    fn new(max_tokens: u32, refill_rate_ms: u64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            last_refill: Instant::now(),
            refill_rate_ms,
        }
    }

    fn check(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_refill).as_millis() as u64;
        if elapsed_ms >= self.refill_rate_ms {
            let refills = (elapsed_ms / self.refill_rate_ms) as u32;
            self.tokens = (self.tokens + refills).min(self.max_tokens);
            self.last_refill = now;
        }
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// Reads from one player's WebSocket and forwards to the other player's output channel.
/// When this player disconnects, sends OpponentDisconnected to the peer.
pub async fn relay_session(peer_tx: &mpsc::Sender<String>, my_rx: &mut WsRx) {
    let mut limiter = RateLimit::new(RATE_LIMIT_BURST, RATE_LIMIT_REFILL_MS);

    while let Some(msg_result) = my_rx.next().await {
        let msg = match msg_result {
            Ok(Message::Text(t)) => t.to_string(),
            Ok(Message::Close(_)) => {
                eprintln!("[relay] peer sent Close frame");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                eprintln!("[relay] WebSocket read error: {e}");
                break;
            }
        };

        // Rate limit check — disconnect on exceeded
        if !limiter.check() {
            eprintln!("[relay] rate limit exceeded, disconnecting peer");
            break;
        }

        // Validate message is a valid PeerMessage before forwarding
        if serde_json::from_str::<PeerMessage>(&msg).is_err() {
            eprintln!("[relay] invalid PeerMessage, skipping");
            continue;
        }

        let envelope =
            serde_json::to_string(&ServerRelayMessage::PeerEnvelope { payload: msg }).unwrap();

        // Use send().await for backpressure instead of try_send() which would
        // disconnect players when the channel is momentarily full
        if peer_tx.send(envelope).await.is_err() {
            eprintln!("[relay] peer channel closed, ending session");
            break;
        }
    }

    // Notify the peer that this player disconnected
    let _ = peer_tx
        .send(serde_json::to_string(&ServerRelayMessage::OpponentDisconnected).unwrap())
        .await;
}
