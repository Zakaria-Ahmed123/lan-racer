use anyhow::Result;
use std::net::Ipv4Addr;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tun_rs::DeviceBuilder;

use crate::event::LanEvent;
use crate::peer::PeerManager;

pub struct Router {}

impl Router {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn route(&self, token: CancellationToken) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(32);
        let manager = PeerManager::new(tx.clone()).await?;

        let args: Vec<_> = std::env::args().collect();

        let dev_name = args.get(1).map(|s| s.as_str()).unwrap_or("tun0");
        let local_ip = args.get(2).map(|s| s.as_str()).unwrap_or("10.10.0.1");
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        let dev = DeviceBuilder::new()
            .name(dev_name)
            .mtu(1500)
            .ipv4(local_ip, mask, None)
            .build_async()?;

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

        tokio::select! {
            _ = mainloop => {
                println!("the mainloop exited to early");
            },
            _ = recvloop => {
                println!("the recvloop exited to early");
            },
            _ = token.cancelled() => {
                println!("Bye!!");
            }
        };

        Ok(())
    }
}
