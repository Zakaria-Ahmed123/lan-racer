#[derive(Debug)]
pub enum LanEvent {
    PacketFromPeer(Vec<u8>),
    NewPeerOffer(String, String),
    PeerConnected(String),
    PeerDisconnected(String),
}
