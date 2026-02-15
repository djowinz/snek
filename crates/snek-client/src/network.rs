use color_eyre::Result;
use futures_util::{SinkExt, StreamExt};
use snek_shared::{ClientRelayMessage, MAX_WS_FRAME_SIZE, MAX_WS_MESSAGE_SIZE, PeerMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

pub struct NetworkConnection {
    pub tx: mpsc::UnboundedSender<String>,
    pub rx: mpsc::UnboundedReceiver<String>,
}

impl NetworkConnection {
    pub async fn connect(url: &str) -> Result<Self> {
        let mut ws_config = WebSocketConfig::default();
        ws_config.max_message_size = Some(MAX_WS_MESSAGE_SIZE);
        ws_config.max_frame_size = Some(MAX_WS_FRAME_SIZE);
        let (ws_stream, _) = connect_async_with_config(url, Some(ws_config), false).await?;
        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
        let (in_tx, in_rx) = mpsc::unbounded_channel::<String>();

        // Writer task: channel → WebSocket
        tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: WebSocket → channel
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                if let Message::Text(text) = msg
                    && in_tx.send(text.to_string()).is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            tx: out_tx,
            rx: in_rx,
        })
    }

    pub fn send_relay(&self, msg: &ClientRelayMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        self.tx
            .send(json)
            .map_err(|e| color_eyre::eyre::eyre!("Send failed: {e}"))?;
        Ok(())
    }

    pub fn send_peer(&self, msg: &PeerMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        self.tx
            .send(json)
            .map_err(|e| color_eyre::eyre::eyre!("Send failed: {e}"))?;
        Ok(())
    }
}
