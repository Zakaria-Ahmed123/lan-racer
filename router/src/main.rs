mod config;
mod event;
mod peer;

use anyhow::Result;
use std::net::Ipv4Addr;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tun_rs::DeviceBuilder;

use crate::event::LanEvent;
use crate::peer::PeerManager;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::select! {
        r = app() => { r.unwrap() },
        _ = tokio::signal::ctrl_c() => {}
    };

    Ok(())
}

async fn app() -> Result<()> {
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

    let cliloop = async {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut line = String::new();
        println!("The lan-racer cli interface type help for commands");
        loop {
            stdout.write(b">>> ").await.unwrap();
            stdout.flush().await.unwrap();
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            let cmd = line.trim();
            match cmd {
                "quit" => break,
                "help" => println!("commands:\noffer\nanswer\nquit\nhelp"),
                "offer" => {
                    stdout.write_all(b"enter peer name\n").await.unwrap();
                    stdout.flush().await.unwrap();
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    let peer_id = line.trim();
                    if manager.has_peer(peer_id).await {
                        println!("Peer with same name exist");
                        continue;
                    }
                    let offer = manager.create_offer(peer_id.into()).await.unwrap();
                    println!("This is the offer:\n{offer}");
                    println!("=========== Past the answer ===========");
                    let mut answer = String::new();
                    reader.read_line(&mut answer).await.unwrap();
                    manager
                        .set_answer_as_offerer(peer_id, &answer)
                        .await
                        .unwrap();
                }
                "answer" => {
                    print!("enter peer name");
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    let peer_id = line.trim();
                    if manager.has_peer(peer_id).await {
                        println!("Peer with same name exist");
                        continue;
                    }
                    println!("=========== Past the offer ===========");
                    let mut offer = String::new();
                    reader.read_line(&mut offer).await.unwrap();
                    let answer = manager.accept_offer(peer_id.into(), &offer).await.unwrap();
                    println!("This is the answer:\n{answer}");
                }
                _ => {
                    println!("unknow command \"{cmd}\"");
                    continue;
                }
            };
        }
    };

    tokio::select! {
        _ = mainloop => {
            println!("the mainloop exited to early");
        },
        _ = recvloop => {
            println!("the recvloop exited to early");
        },
        _ = cliloop => {
            println!("the cliloop exited to early");
        }
    };

    Ok(())
}
