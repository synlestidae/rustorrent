use wire::handler::{PeerId, PeerAction, ServerHandler};
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;
use file::PartialFile;
use std::collections::HashMap;
use wire::data::PeerMsg;
use std::time::SystemTime;

pub struct PeerState {
    has_handshake: bool,
    disconnected: bool,
    peer_choking: bool,
    peer_interested: bool,
    am_choking: bool,
    am_interested: bool,
    last_keepalive: SystemTime
}

impl PeerState {
    pub fn new() -> PeerState {
        PeerState {
            has_handshake: false,
            disconnected: false,
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_interested: false,
            last_keepalive: SystemTime::now()
        }
    }
}

pub struct PeerServer {
    peers: HashMap<PeerId, PeerState>, 
    hash: SHA1Hash20b,
    our_peer_id: String,
    partial_file: PartialFile
}

const PROTOCOL_ID: &'static str = "rustorrent-beta";

impl ServerHandler for PeerServer {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self {
        let partial_file = PartialFile::new(&metainfo.info);

        PeerServer{ 
            peers: HashMap::new(), 
            hash: hash,
            our_peer_id: our_peer_id.to_string(),
            partial_file: partial_file
        }
    }

    fn on_peer_connect(&mut self, id: PeerId) -> PeerAction {
        let handshake = PeerMsg::handshake(PROTOCOL_ID.to_string(), self.our_peer_id.to_string(), &self.hash);
        PeerAction::SendMessages(vec![handshake])
    }

    fn on_message_receive(&mut self, id: PeerId, msg: PeerMsg) -> PeerAction {
        let peer = match self.peers.get_mut(&id) {
            Some(peer) => peer,
            None => return PeerAction::Nothing
        };

        if peer.disconnected {
            return PeerAction::Nothing;
        }

        if !peer.has_handshake {
            match msg {
                PeerMsg::HandShake(_, ref their_hash, _) => {
                    if their_hash == &self.hash {
                        peer.has_handshake = true;
                    } else {
                        peer.disconnected = false;
                    }
                    return PeerAction::Nothing;
                },
                _ => peer.disconnected = true
            }
        }

        match msg {
            PeerMsg::HandShake(..) => {},
            PeerMsg::KeepAlive => {},
            PeerMsg::Choke => peer.peer_choking = true,
            PeerMsg::Unchoke=> peer.peer_choking = false,
            PeerMsg::Interested => peer.peer_interested = true,
            PeerMsg::NotInterested => peer.peer_interested = false,
            PeerMsg::Have(_) => {},
            PeerMsg::Bitfield(_) => {},
            PeerMsg::Request(..) => {},
            PeerMsg::Piece(..) => {},
            PeerMsg::Cancel(..) => {},
            PeerMsg::Port(_) => {}
        }
        
        PeerAction::Nothing
    }

    fn on_peer_disconnect(&mut self, id: PeerId) -> PeerAction {
        unimplemented!();
    }

    fn on_loop(&mut self) -> PeerAction {
        unimplemented!();
    }
}
