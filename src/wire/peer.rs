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
use wire::peer_info::PeerState;
use wire::strategy::{Strategy, NormalStrategy, Order, OrderResult};

const TIMEOUT_SECONDS: u64 = 60 * 5;
const KEEPALIVE_PERIOD: u64 = 30;

pub struct PeerServer {
    peers: HashMap<PeerId, Peer>,
    hash: SHA1Hash20b,
    our_peer_id: String,
    partial_file: PartialFile,
    num_pieces: usize,
    pieces_to_request: BitVec,
    strategy: NormalStrategy
}

struct Peer {
    state: PeerState, 
    orders: Vec<Order>
}

impl Peer {
    pub fn new(state: PeerState) -> Peer {
        Peer { state: state, orders: Vec::new() } 
    }
}

const PROTOCOL_ID: &'static str = "BitTorrent protocol";

impl ServerHandler for PeerServer {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self {
        let num_pieces = metainfo.info.pieces.len();
        let partial_file = PartialFile::new(&metainfo.info);
        let pl = partial_file.piece_length();

        PeerServer {
            peers: HashMap::new(),
            hash: hash,
            our_peer_id: our_peer_id.to_string(),
            partial_file: partial_file,
            num_pieces: num_pieces,
            pieces_to_request: BitVec::from_elem(num_pieces, true),
            strategy: NormalStrategy::new(metainfo)
        }
    }

    fn on_peer_connect(&mut self, id: PeerId) -> PeerAction {
        let peer = Peer::new(PeerState::new(self.num_pieces, id));
        self.peers.insert(id, peer);

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
        let orders = self.strategy.query(Vec::new(), 
            Vec::new(), self.peers.iter().map(|(_, p)| &p.state).collect::<Vec<_>>(), &self.partial_file);

        self._execute_orders(orders)

    }
}

impl PeerServer {
    fn _execute_orders(&mut self, orders: Vec<Order>) -> Vec<PeerAction> {
        let mut actions = Vec::new();
        for order in orders.into_iter() {
            actions.push(order.action);
        }
        actions
    }

    fn _remove_old_peers(&mut self) {
        let for_removal = self._get_timeout_ids();
        for id in for_removal {
            self.peers.remove(&id);
        }
    }

    fn _get_timeout_ids(&self) -> Vec<PeerId> {
        let mut for_removal = Vec::new();
        for (&id, ref mut peer) in &self.peers {
            match peer.state.last_msg_time.elapsed() {
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
        for (&id, ref mut peer) in &self.peers {
            match peer.state.last_msg_time.elapsed() {
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

            let peer = &mut match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerStreamAction::Nothing,
            }.state;

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
                            peer.am_choking = false;
                            return PeerStreamAction::SendMessages(vec![PeerMsg::Unchoke,
                                                                       PeerMsg::Interested,
                                                                       PeerMsg::Bitfield(self.partial_file.bit_array())]);
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
            let peer = &mut match self.peers.get_mut(&id) {
                Some(peer) => peer,
                None => return PeerStreamAction::Nothing,
            }.state;

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
                PeerMsg::Bitfield(ref bit_field) => {
                    let limit = peer.file.bit_array().len();
                    for (i, bit) in bit_field.iter().enumerate()  {
                        if i >= limit {
                            break;
                        }
                        peer.file.set(i, bit);
                    }
                }
                _ => handled = false,
            };
        }

        // messages that don't need to mutate peer
        let choking = {
            match self.peers.get(&id) {
                Some(peer) => peer.state.am_choking,
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