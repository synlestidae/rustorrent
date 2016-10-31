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
use wire::strategy::{Strategy, NormalStrategy, Order};

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
        let orders = self.strategy.query();

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
        //handshake is okay
        let orders = match msg {
            PeerMsg::HandShake(_, _, _) => self.strategy.on_handshake(id),
            PeerMsg::KeepAlive => return PeerStreamAction::Nothing,
            PeerMsg::Choke => self.strategy.on_choke(id),
            PeerMsg::Unchoke => self.strategy.on_unchoke(id),
            PeerMsg::Interested => self.strategy.on_interested(id),
            PeerMsg::NotInterested => self.strategy.on_not_interested(id),
            PeerMsg::Have(pi) => self.strategy.on_have(id, pi as usize),
            PeerMsg::Bitfield(bit_vec) => self.strategy.on_bitfield(id, bit_vec),
            PeerMsg::Request(index, begin, length) => self.strategy.on_request(id, index, begin, length),
            PeerMsg::Piece(index, begin, block) => self.strategy.on_piece(id, index, begin, block),
            PeerMsg::Cancel(index, begin, block) =>  return PeerStreamAction::Nothing,
            //self.strategy.on_cancel(id, index, begin, block),
            PeerMsg::Port(port) => self.strategy.on_port(id, port as u16)
        };

        //let actions = orders.into_iter().map(|order| order.action.1);
        //actions.collect()
        let mut msgs = Vec::new();
        for order in orders {
            match order.action.1 {
                PeerStreamAction::SendMessages(mut msgs_to_send) => {
                    msgs.append(&mut msgs_to_send);
                },
                _ => ()
            }
        }
        PeerStreamAction::SendMessages(msgs)
    }
}
