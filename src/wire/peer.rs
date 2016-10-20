use wire::handler::ServerHandler;
use wire::action::{PeerId, PeerStreamAction, PeerAction};
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;
use file::PartialFile;
use std::collections::HashMap;
use wire::msg::PeerMsg;
use std::time::SystemTime;
use file::{PartialFileTrait, PeerFile};
use bit_vec::BitVec;

const TIMEOUT_SECONDS: u64 = 60 * 5;
const KEEPALIVE_PERIOD: u64 = 30;

struct PeerState {
    has_handshake: bool,
    disconnected: bool,
    peer_choking: bool,
    peer_interested: bool,
    am_choking: bool,
    am_interested: bool,
    last_msg_time: SystemTime,
    last_msg_sent_time: SystemTime,
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
            last_msg_time: SystemTime::now(),
            last_msg_sent_time: SystemTime::now(),
            file: PeerFile::new(len),
        }
    }
}

pub struct PeerServer {
    peers: HashMap<PeerId, PeerState>,
    hash: SHA1Hash20b,
    our_peer_id: String,
    partial_file: PartialFile,
    num_pieces: usize,
    pieces_to_request: BitVec
}

const PROTOCOL_ID: &'static str = "BitTorrent protocol";

impl ServerHandler for PeerServer {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self {
        let num_pieces = metainfo.info.pieces.len();
        let partial_file = PartialFile::new(&metainfo.info);

        PeerServer {
            peers: HashMap::new(),
            hash: hash,
            our_peer_id: our_peer_id.to_string(),
            partial_file: partial_file,
            num_pieces: num_pieces,
            pieces_to_request: BitVec::from_elem(num_pieces, true)
        }
    }

    fn on_peer_connect(&mut self, id: PeerId) -> PeerAction {
        self.peers.insert(id, PeerState::new(self.num_pieces));

        let handshake = PeerMsg::handshake(PROTOCOL_ID.to_string(),
                                           self.our_peer_id.to_string(),
                                           &self.hash);

        PeerAction(id, PeerStreamAction::SendMessages(vec![handshake]))
    }

    fn on_message_receive(&mut self, id: PeerId, msg: PeerMsg) -> PeerAction {
        let msg = self._on_message_receive(id, msg);
        PeerAction(id, msg)
    }

    fn on_peer_disconnect(&mut self, id: PeerId) -> PeerAction {
        self.peers.remove(&id);
        PeerAction(id, PeerStreamAction::Nothing)
    }

    // remove peers that have not replied in five minutes
    fn on_loop(&mut self) -> Vec<PeerAction> {
        info!("We have {} peers", self.peers.len());
        self._remove_old_peers();
        let request_actions = self._request_pieces();
        info!("Sending {} requests", request_actions.len());
        request_actions
    }
}

impl PeerServer {
    fn _remove_old_peers(&mut self) {
        let for_removal = self._get_timeout_ids();
        for id in for_removal {
            self.peers.remove(&id);
        }
    }

    fn _request_pieces(&mut self) -> Vec<PeerAction> {
        let mut actions = Vec::new();
        for (&id, peer) in &self.peers {
            // Skip choking peers
            if peer.peer_choking || peer.am_choking || !peer.has_handshake {
                continue;
            }

            let mut missing = self.partial_file.bit_array();
            missing.negate();
            let mut them = peer.file.bit_array();
            missing.intersect(&them);
            missing.intersect(&self.pieces_to_request);

            //now have eligible pieces to request
            let mut pieces = missing.into_iter().enumerate().filter(|&(_, x)| x).map(|(i, _)| i as u64).collect::<Vec<u64>>();
            let piece_len = self.partial_file.piece_length();
            let mut bytes_requested = 0;
            let mut pieces_request = 0;

            const MAX_BLOCK_SIZE: u64 = 2 << 14;
            const MAX_BYTES_PER_REQUEST: u64 = 1024 * 512;
            const MAX_PIECES_PEER: usize = 10;

            let mut msgs = Vec::new();
            for (piece_count, piece_index) in pieces.into_iter().enumerate() {
                if bytes_requested > MAX_BYTES_PER_REQUEST || piece_count > MAX_PIECES_PEER {
                    break;
                }
                if piece_len > MAX_BLOCK_SIZE {
                    let mut offset = 0;
                    while offset < piece_len {
                        if (offset + MAX_BLOCK_SIZE > piece_len) {
                            msgs.push(PeerMsg::Request(piece_index as u32, offset as u32, piece_len as u32));
                        } else {
                            msgs.push(PeerMsg::Request(piece_index as u32, offset as u32, piece_len as u32));
                        }
                        offset += MAX_BLOCK_SIZE;
                    }
                } else {
                    msgs.push(PeerMsg::Request(piece_index as u32, 0, piece_len as u32));
                }
            }

            let req_actions = PeerAction(id, PeerStreamAction::SendMessages(msgs));
            actions.push(req_actions);
        }
        actions
    }


    fn _get_timeout_ids(&self) -> Vec<PeerId> {
        let mut for_removal = Vec::new();
        for (&id, peer) in &self.peers {
            match peer.last_msg_time.elapsed() {
                Ok(duration) => {
                    if duration.as_secs() > TIMEOUT_SECONDS {
                        for_removal.push(id);
                    }
                }
                _ => (),
            }
        }
        for_removal
    }

    fn _get_keepalive_ids(&self) -> Vec<PeerId> {
        let mut for_keeping = Vec::new();
        for (&id, peer) in &self.peers {
            match peer.last_msg_time.elapsed() {
                Ok(duration) => {
                    if duration.as_secs() > KEEPALIVE_PERIOD {
                        for_keeping.push(id);
                    }
                }
                _ => (),
            }
        }
        for_keeping
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

    fn _on_message_receive(&mut self, id: PeerId, msg: PeerMsg) -> PeerStreamAction {
        {
            info!("Received msg {:?} from id {}", msg, id);

            let peer = match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerStreamAction::Nothing,
            };

            if peer.disconnected {
                return PeerStreamAction::Nothing;
            }

            peer.last_msg_time = SystemTime::now();

            if !peer.has_handshake {
                info!("Do not have handshake yet");
                match msg {
                    PeerMsg::HandShake(_, ref their_hash, _) => {
                        if their_hash == &self.hash {
                            info!("Hashes match, sending interested message now");
                            peer.has_handshake = true;
                            return PeerStreamAction::SendMessages(vec![PeerMsg::Unchoke,
                                                                       PeerMsg::Interested]);
                        } else {
                            peer.disconnected = true;
                            info!("Handshake info hash does not match");
                            return PeerStreamAction::Disconnect;
                        }
                    }
                    _ => {
                        peer.disconnected = true;
                    }
                }
            }
        }

        let mut outgoing_msgs = Vec::new();
        let mut handled = true;

        {
            let peer = match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerStreamAction::Nothing,
            };

            // messages that mutate the peer
            match msg {
                PeerMsg::HandShake(..) => {
                    peer.has_handshake = true;
                }
                PeerMsg::KeepAlive => {}
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
                None => return PeerStreamAction::Nothing,
            }
        };

        if !choking && !handled {
            match msg {
                PeerMsg::Request(index, begin, offset) => {
                    let response = self._get_piece_from_req(index as usize, begin, offset);
                    match response {
                        Some(r) => outgoing_msgs.push(r),
                        _ => (),
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

        info!("We have messages to send: {:?}", outgoing_msgs);

        if outgoing_msgs.len() > 0 {
            PeerStreamAction::SendMessages(outgoing_msgs)
        } else {
            PeerStreamAction::Nothing
        }
    }
}
