use anyhow::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
};

use std::{collections::HashMap, sync::Arc};

use crate::signaling::protocol::SignalMessage;

type PeerMap = Arc<RwLock<HashMap<String, mpsc::Sender<SignalMessage>>>>;

pub async fn run_server(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));

    println!("Signaling server listening on {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;

        let peers = peers.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_peer(socket, peers).await {
                eprintln!("Peer error: {}", e);
            }
        });
    }
}

async fn handle_peer(
    socket: TcpStream,
    peers: PeerMap,
) -> Result<()> {
    let (reader, mut writer) = socket.into_split();

    let mut lines = BufReader::new(reader).lines();

    let (tx, mut rx) = mpsc::channel::<SignalMessage>(32);

    let mut my_peer_id = None;

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            writer.write_all(json.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
    });

    while let Some(line) = lines.next_line().await? {
        let msg: SignalMessage = serde_json::from_str(&line)?;

        match &msg {
            SignalMessage::Register { peer_id } => {
                peers.write().await.insert(peer_id.clone(), tx.clone());

                my_peer_id = Some(peer_id.clone());

                println!("Registered {}", peer_id);
            }

            SignalMessage::Offer { to, .. }
            | SignalMessage::Answer { to, .. } => {
                if let Some(target) = peers.read().await.get(to) {
                    target.send(msg).await?;
                }
            }

            SignalMessage::Chat { .. } => {}
        }
    }

    if let Some(id) = my_peer_id {
        peers.write().await.remove(&id);
    }

    Ok(())
}