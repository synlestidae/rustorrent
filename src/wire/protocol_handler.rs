use wire::handler::{PeerId, PeerAction, ServerHandler};
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;
use file::PartialFile;
use std::collections::HashMap;
use wire::data::PeerMsg;
use std::time::SystemTime;
use file::{PartialFileTrait, PeerFile};

const TIMEOUT_SECONDS: u64 = 60 * 5;

pub struct PeerState {
    has_handshake: bool,
    disconnected: bool,
    peer_choking: bool,
    peer_interested: bool,
    am_choking: bool,
    am_interested: bool,
    last_keepalive: SystemTime,
    file: PeerFile,
}

impl PeerState {
    pub fn new(len: usize) -> PeerState {
        PeerState {
            has_handshake: false,
            disconnected: false,
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_interested: false,
            last_keepalive: SystemTime::now(),
            file: PeerFile::new(len),
        }
    }
}

pub struct PeerServer {
    peers: HashMap<PeerId, PeerState>,
    hash: SHA1Hash20b,
    our_peer_id: String,
    partial_file: PartialFile,
}

const PROTOCOL_ID: &'static str = "rustorrent-beta";

impl ServerHandler for PeerServer {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self {
        let partial_file = PartialFile::new(&metainfo.info);

        PeerServer {
            peers: HashMap::new(),
            hash: hash,
            our_peer_id: our_peer_id.to_string(),
            partial_file: partial_file,
        }
    }

    fn on_peer_connect(&mut self, id: PeerId) -> PeerAction {
        let handshake = PeerMsg::handshake(PROTOCOL_ID.to_string(),
                                           self.our_peer_id.to_string(),
                                           &self.hash);
        PeerAction::SendMessages(vec![handshake])
    }

    fn on_message_receive(&mut self, id: PeerId, msg: PeerMsg) -> PeerAction {
        {
            let peer = match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerAction::Nothing,
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
                    }
                    _ => peer.disconnected = true,
                }
            }
        }

        let mut outgoing_msgs = Vec::new();
        let mut handled = true;

        {
            let peer = match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerAction::Nothing,
            };

            // messages that mutate the peer
            match msg {
                PeerMsg::HandShake(..) => {}
                PeerMsg::KeepAlive => peer.last_keepalive = SystemTime::now(),
                PeerMsg::Choke => {
                    peer.peer_choking = true;
                }
                PeerMsg::Unchoke => {
                    peer.peer_choking = false;
                }
                PeerMsg::Interested => {
                    peer.peer_interested = true;
                }
                PeerMsg::NotInterested => {
                    peer.peer_interested = false;
                }
                PeerMsg::Have(index) => peer.file.set(index as usize, true),
                PeerMsg::Bitfield(_) => {}
                _ => handled = false,
            };
        }

        // messages that don't need to mutate peer
        let choking = {
            match self.peers.get(&id) {
                Some(peer) => peer.am_choking,
                None => return PeerAction::Nothing,
            }
        };

        if !choking && !handled {
            match msg {
                PeerMsg::Request(index, begin, offset) => {
                    let response = self._get_piece_from_req(index as usize, begin, offset);
                    if response.is_some() {
                        outgoing_msgs.push(response.unwrap());
                    }
                }
                PeerMsg::Piece(index, begin, block) => {
                    self.partial_file.add_piece(index as usize, begin as usize, block);
                }
                PeerMsg::Cancel(..) => {}
                PeerMsg::Port(_) => {}
                _ => handled = true,
            }
        }

        if outgoing_msgs.len() > 0 {
            PeerAction::SendMessages(outgoing_msgs)
        } else {
            PeerAction::Nothing
        }
    }

    fn on_peer_disconnect(&mut self, id: PeerId) -> PeerAction {
        self.peers.remove(&id);
        PeerAction::Nothing
    }

    //remove peers that have no replied in five minutes
    fn on_loop(&mut self) -> PeerAction {
        self._remove_old_peers();
        PeerAction::Nothing
    }
}

impl PeerServer {
    fn _remove_old_peers(&mut self) {

        let for_removal = self._get_timeout_ids();

        for id in for_removal {
            self.peers.remove(&id);
        }

    }

    fn _get_timeout_ids(&self) -> Vec<PeerId> {
        let mut for_removal = Vec::new();
            for (&id, peer) in &self.peers {
                match peer.last_keepalive.elapsed() {
                    Ok(duration) => {
                        if duration.as_secs() > TIMEOUT_SECONDS {
                            for_removal.push(id);
                        }
                    },
                    _ => ()
                }
            }
            for_removal
    }

    fn _get_piece_from_req(&mut self, index: usize, begin: u32, offset: u32) -> Option<PeerMsg> {
        if self.partial_file.has_piece(index as usize) {
            let piece = self.partial_file.get_piece_mut(index as usize);
            return match piece.get_offset(begin as usize, offset as usize) {
                Some(piece_data) => {
                    let msg = PeerMsg::Piece(begin, offset, Vec::from(piece_data));
                    Some(msg)
                }
                _ => None,
            };
        }
        None
    }
}
