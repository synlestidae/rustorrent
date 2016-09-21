use wire::handler::{PeerId, PeerAction, ServerHandler}

pub struct PeerState {
    has_handshake: bool,
    peer_choking: bool,
    peer_interested: bool,
    am_choking: bool,
    am_interested: bool
}

impl PeerState {
    pub fn new() -> PeerState {
        PeerState {
            has_handshake: false,
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_interested: false
        }
    }
}
