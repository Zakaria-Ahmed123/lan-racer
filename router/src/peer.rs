use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::{API, APIBuilder};
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::event::LanEvent;

#[derive(Clone)]
pub struct PeerManager {
    api: Arc<API>,
    peers: Arc<RwLock<HashMap<String, Arc<RTCPeerConnection>>>>,
    data_channels: Arc<RwLock<HashMap<String, Arc<RTCDataChannel>>>>,
    event_tx: mpsc::Sender<LanEvent>,
}

impl PeerManager {
    pub async fn new(event_tx: mpsc::Sender<LanEvent>) -> Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;

        let mut registry = webrtc::interceptor::registry::Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        Ok(Self {
            api: Arc::new(api),
            peers: Arc::new(RwLock::new(HashMap::new())),
            data_channels: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        })
    }

    pub async fn has_peer(&self, peer_id: &str) -> bool {
        self.peers.read().await.contains_key(peer_id)
    }

    async fn new_peer_connection(&self, peer_id: String) -> Result<Arc<RTCPeerConnection>> {
        let config = RTCConfiguration {
            ice_servers: vec![webrtc::ice_transport::ice_server::RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let pc = Arc::new(self.api.new_peer_connection(config).await?);
        let event_tx_clone = self.event_tx.clone();
        let pid_clone = peer_id.clone();

        pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            let tx = event_tx_clone.clone();
            let pid = pid_clone.clone();
            Box::pin(async move {
                if s == RTCPeerConnectionState::Connected {
                    let _ = tx.send(LanEvent::PeerConnected(pid)).await;
                } else if s == RTCPeerConnectionState::Failed || s == RTCPeerConnectionState::Closed
                {
                    let _ = tx.send(LanEvent::PeerDisconnected(pid)).await;
                }
            })
        }));

        let mut peers = self.peers.write().await;
        peers.insert(peer_id, pc.clone());

        Ok(pc)
    }

    pub async fn create_offer(&self, peer_id: String) -> Result<String> {
        let pc = self.new_peer_connection(peer_id.clone()).await?;

        let dc = pc.create_data_channel("chat", None).await?;
        self.setup_data_channel(&dc, peer_id.clone()).await;

        let offer = pc.create_offer(None).await?;

        let mut gather_complete = pc.gathering_complete_promise().await;
        pc.set_local_description(offer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = pc.local_description().await.ok_or(anyhow!("No SDP"))?;
        let json = serde_json::to_string(&local_desc)?;
        Ok(json)
    }

    pub async fn set_answer_as_offerer(&self, peer_id: &str, answer_json: &str) -> Result<()> {
        let peers = self.peers.read().await;
        let pc = peers.get(peer_id).ok_or(anyhow!("Peer not found"))?;

        let answer = serde_json::from_str::<RTCSessionDescription>(answer_json)?;
        pc.set_remote_description(answer).await?;
        Ok(())
    }

    pub async fn accept_offer(&self, peer_id: String, offer_json: &str) -> Result<String> {
        let pc = self.new_peer_connection(peer_id.clone()).await?;

        let manager_clone = self.clone();
        let pid_clone = peer_id.clone();

        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            let manager = manager_clone.clone();
            let pid = pid_clone.clone();
            Box::pin(async move {
                manager.setup_data_channel(&dc, pid).await;
            })
        }));

        let offer = serde_json::from_str::<RTCSessionDescription>(offer_json)?;
        pc.set_remote_description(offer).await?;

        let answer = pc.create_answer(None).await?;
        let mut gather_complete = pc.gathering_complete_promise().await;
        pc.set_local_description(answer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = pc.local_description().await.ok_or(anyhow!("No SDP"))?;
        let json = serde_json::to_string(&local_desc)?;
        Ok(json)
    }

    async fn setup_data_channel(&self, dc: &Arc<RTCDataChannel>, peer_id: String) {
        let dc_clone = dc.clone();
        let tx = self.event_tx.clone();

        dc.on_message(Box::new(move |msg: DataChannelMessage| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(LanEvent::PacketFromPeer(msg.data.to_vec()))
                    .await
                    .unwrap();
            })
        }));

        let mut channels = self.data_channels.write().await;
        channels.insert(peer_id, dc_clone);
    }

    pub async fn route_and_send(&self, pkt: Vec<u8>) -> Result<()> {
        let bytes = bytes::Bytes::from_owner(pkt);
        for (_, chan) in self.data_channels.read().await.iter() {
            chan.send(&bytes).await?;
        }
        Ok(())
    }
}
