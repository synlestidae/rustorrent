use std::time::SystemTime;
use file::PeerFile;
use wire::action::PeerId;

pub struct PeerState {
    pub peer_id: PeerId,
    pub has_handshake: bool,
    pub disconnected: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
    pub am_choking: bool,
    pub am_interested: bool,
    pub last_msg_time: SystemTime,
    pub last_msg_sent_time: SystemTime,
    pub file: PeerFile,
    pub connection_time: SystemTime,
}

impl PeerState {
    pub fn new(len: usize, id: PeerId) -> PeerState {
        PeerState {
            peer_id: id,
            has_handshake: false,
            disconnected: false,
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_interested: false,
            last_msg_time: SystemTime::now(),
            last_msg_sent_time: SystemTime::now(),
            connection_time: SystemTime::now(),
            file: PeerFile::new(len),
        }
    }
}
