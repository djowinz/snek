mod lobby;
mod session;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use color_eyre::Result;
use futures_util::StreamExt;
use snek_shared::{MAX_WS_FRAME_SIZE, MAX_WS_MESSAGE_SIZE};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

use lobby::Lobby;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let addr = "127.0.0.1:9090";
    let listener = TcpListener::bind(addr).await?;
    println!("snek-relay listening on {addr}");

    let lobby = Arc::new(Lobby::new());

    let mut ws_config = WebSocketConfig::default();
    ws_config.max_message_size = Some(MAX_WS_MESSAGE_SIZE);
    ws_config.max_frame_size = Some(MAX_WS_FRAME_SIZE);

    loop {
        let (stream, peer_addr) = listener.accept().await?;

        let current = lobby.active_connections.load(Ordering::Relaxed);
        if current >= snek_shared::MAX_RELAY_CONNECTIONS {
            eprintln!("Max connections reached, rejecting {peer_addr}");
            drop(stream);
            continue;
        }
        lobby.active_connections.fetch_add(1, Ordering::Relaxed);

        println!("New connection from {peer_addr}");
        let lobby = Arc::clone(&lobby);

        tokio::spawn(async move {
            match accept_async_with_config(stream, Some(ws_config)).await {
                Ok(ws_stream) => {
                    let (ws_tx, ws_rx) = ws_stream.split();
                    lobby.handle_client(ws_tx, ws_rx, peer_addr).await;
                }
                Err(e) => {
                    eprintln!("WebSocket handshake failed for {peer_addr}: {e}");
                }
            }
            lobby.active_connections.fetch_sub(1, Ordering::Relaxed);
        });
    }
}
