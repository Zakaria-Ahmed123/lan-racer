#[allow(dead_code)]
use anyhow::Result;
use std::net::Ipv4Addr;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tun_rs::DeviceBuilder;

use crate::event::LanEvent;
use crate::peer::PeerManager;

#[derive(Debug)]
pub enum RouterCommand{ 
    CreateOffer { peer_id: String },
    AcceptOffer { peer_id: String, sdp: String },
    CreateAnswer { peer_id: String, sdp: String },
    ConnectToPeer { peer_id: String }, 
    SendChat { peer_id: String, message: String },
}
pub struct Router; 

impl Router {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn route(&self, token: CancellationToken,mut cmd_rx: mpsc::Receiver<RouterCommand>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(32);
        let manager = PeerManager::new(tx.clone()).await?;

        use crate::signaling::client::SignalClient;
        use crate::signaling::protocol::SignalMessage;

        
        let args: Vec<_> = std::env::args().collect();


        let dev_name = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tun1");

        let local_ip = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("10.10.0.2");

        let my_id = args
        .get(3)
        .cloned()
        .unwrap_or("peer1".into());
        
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        let dev = DeviceBuilder::new()
            .name(dev_name)
            .mtu(1500)
            .ipv4(local_ip, mask, None)
            .build_async()?;

        let (signal_client, mut signal_rx) =
            SignalClient::connect(
            "127.0.0.1:9000",
            my_id.clone(),
         )
        .await?;

        let recvloop = async {
            let mut buf = vec![0u8; 1500];
            loop {
                let len = dev.recv(&mut buf).await.unwrap();
                if len > 0 {
                    let packet = buf[..len].to_vec();
                    if let Err(e) = manager.route_and_send(packet).await {
                        eprintln!("Error routing packet: {}", e);
                    }
                }
            }
        };

        let mainloop = async {
            loop {
                match rx.recv().await {
                    Some(LanEvent::PacketFromPeer(packet)) => {
                        if let Err(e) = dev.send(&packet).await {
                            eprintln!("Error writing to TUN: {}", e);
                        }
                    }
                    Some(LanEvent::PeerConnected(pid)) => {
                        println!("[System]: Peer {} connected.", pid);
                    }
                    Some(LanEvent::PeerDisconnected(pid)) => {
                        println!("[System]: Peer {} disconnected.", pid);
                    }
                    Some(LanEvent::NewPeerOffer(pid, sdp)) => {
                        println!("\n--- RECEIVED OFFER from {pid} ---");
                        println!("{sdp}");
                    }
                    None => break,
                }
            }
        };

    let command_loop = async {
      while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            RouterCommand::CreateOffer { peer_id } => {
                match manager.create_offer(peer_id.clone()).await {
                    Ok(sdp) => {
                        println!("\n=== OFFER for {} ===", peer_id);
                        println!("{sdp}");
                    }
                    Err(e) => eprintln!("Error: {e}"),
                }
            }

            RouterCommand::AcceptOffer { peer_id, sdp } => {
                match manager.accept_offer(peer_id.clone(), &sdp).await {
                    Ok(answer) => {
                        println!("\n=== ANSWER for {} ===", peer_id);
                        println!("{answer}");
                    }
                    Err(e) => eprintln!("Error: {e}"),
                }
            }

            RouterCommand::CreateAnswer { peer_id, sdp } => {
                if let Err(e) = manager
                    .set_answer_as_offerer(&peer_id, &sdp)
                    .await
                {
                    eprintln!("Error: {e}");
                }
            }

            RouterCommand::ConnectToPeer { peer_id } => {
                match manager.create_offer(peer_id.clone()).await {
                    Ok(offer) => {
                    if let Err(e) = signal_client.send(
                        SignalMessage::Offer {
                            from: my_id.clone(),
                            to: peer_id,
                            sdp: offer,
                        }
                    ).await {
                        eprintln!("Signal error: {}", e);
                    }
                    }

                    Err(e) => {
                        eprintln!("Offer error: {}", e);
                    }
                }
            }

            RouterCommand::SendChat { peer_id, message } => {
               if let Err(e) = manager.send_chat(&peer_id, message.clone()).await {
                eprintln!("chat send error: {e}");
                }
            }
        }
    }
};

let signaling_loop = async {
    while let Some(msg) = signal_rx.recv().await {
        match msg {
            SignalMessage::Offer { from, sdp, .. } => {
                match manager.accept_offer(from.clone(), &sdp).await {
                    Ok(answer) => {
                        if let Err(e) = signal_client.send(
                            SignalMessage::Answer {
                                from: my_id.clone(),
                                to: from,
                                sdp: answer,
                            }
                        ).await {
                            eprintln!("Signal error: {}", e);
                        }
                    }

                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                    }
                }
            }

            SignalMessage::Answer { from, sdp, .. } => {
                if let Err(e) =
                    manager.set_answer_as_offerer(&from, &sdp).await
                {
                    eprintln!("Answer error: {}", e);
                }
            }

            _ => {}
        }
    }
};

        tokio::select! {
            _ = mainloop => {
                println!("the mainloop exited to early");
            },
            _ = recvloop => {
                println!("the recvloop exited to early");
            },
            _ = command_loop => { 
                println!("command loop exited"); 
            }
            _ = signaling_loop => {
                println!("signaling_loop exited");
            }
            _ = token.cancelled() => {
                println!("Bye!!");
            }
        };

        Ok(())
    }
}
