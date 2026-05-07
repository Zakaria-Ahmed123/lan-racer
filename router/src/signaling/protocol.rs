use serde::{Serialize, Deserialize};

#[derive(Debug,Clone, Serialize, Deserialize)]
pub enum SignalMessage {
    Register {
        peer_id: String,
    },

    Offer {
        from: String,
        to: String,
        sdp: String,
    },

    Answer {
        from: String,
        to: String,
        sdp: String,
    },

    Chat {
        from: String,
        msg: String,
    },
}