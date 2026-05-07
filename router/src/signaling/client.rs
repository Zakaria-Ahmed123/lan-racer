use anyhow::Result;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{mpsc, Mutex},
};

use std::sync::Arc;

use crate::signaling::protocol::SignalMessage;

#[derive(Clone)]
pub struct SignalClient {
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
}

impl SignalClient {
    pub async fn connect(
        addr: &str,
        peer_id: String,
    ) -> Result<(Self, mpsc::Receiver<SignalMessage>)> {
        let stream = TcpStream::connect(addr).await?;

        let (reader, writer) = stream.into_split();

        let writer = Arc::new(Mutex::new(writer));

        let (tx, rx) = mpsc::channel(32);

        let mut lines = BufReader::new(reader).lines();

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(msg) = serde_json::from_str::<SignalMessage>(&line) {
                    let _ = tx.send(msg).await;
                }
            }
        });

        let client = Self { writer };

        client.send(SignalMessage::Register { peer_id }).await?;

        Ok((client, rx))
    }

    pub async fn send(&self, msg: SignalMessage) -> Result<()> {
        let json = serde_json::to_string(&msg)?;

        let mut writer = self.writer.lock().await;

        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        Ok(())
    }
}