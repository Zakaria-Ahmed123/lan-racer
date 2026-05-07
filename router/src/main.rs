use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

mod router;
mod peer;
mod event;
mod signaling;

use router::{Router, RouterCommand};

#[tokio::main]
async fn main() -> Result<()> {
    let token = CancellationToken::new();

    // Command channel (main → router)
    let (cmd_tx, cmd_rx) = mpsc::channel(32);

    let router = Router::new();

    // Spawn router
    tokio::spawn({
        let token = token.clone();
        async move {
            if let Err(e) = router.route(token, cmd_rx).await {
                eprintln!("Router error: {e}");
            }
        }
    });

    // CLI loop
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    println!("connect <peer_id>");
    println!("chat <peer_id> <message>");
    while let Ok(Some(line)) = lines.next_line().await {
        let parts: Vec<_> = line.splitn(3, ' ').collect();

        match parts.as_slice() {
            ["connect", peer_id] => {
              let _ = cmd_tx
                .send(RouterCommand::ConnectToPeer {
                    peer_id: peer_id.to_string(),
                 })
                .await;
            }

            ["chat", peer_id, msg] => {
              println!("🔥 CHAT COMMAND HIT: {} -> {}", peer_id, msg);
              let _ = cmd_tx
                .send(RouterCommand::SendChat {
                peer_id: peer_id.to_string(),
                message: msg.to_string(),
                })
                .await;
            }

            _ => println!("Unknown command"),
        }
    }

    token.cancel();

    Ok(())
}