use wire::peer_info::PeerState;
use metainfo::MetaInfo;
use wire::action::PeerAction;
use wire::msg::PeerMsg;
use wire::action::PeerStreamAction;
use std::collections::HashMap;
use bit_vec::BitVec;
use file::PartialFile;
use file::PartialFileTrait;
use wire::action::PeerId;
use file::PeerFile;

pub trait Strategy {
    fn on_handshake(&mut self, id: PeerId) -> Vec<Order>;
    fn on_choke(&mut self, id: PeerId) -> Vec<Order>;
    fn on_unchoke(&mut self, id: PeerId) -> Vec<Order>;
    fn on_interested(&mut self, id: PeerId) -> Vec<Order>;
    fn on_not_interested(&mut self, id: PeerId) -> Vec<Order>;
    fn on_have(&mut self, id: PeerId, piece_index: usize) -> Vec<Order>;
    fn on_bitfield(&mut self, id: PeerId, bitfield: BitVec) ->  Vec<Order>;
    fn on_request(&mut self, id: PeerId, index: u32, begin: u32, length: u32) -> Vec<Order>;
    fn on_piece(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order>;
    fn on_cancel(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order>;
    fn on_port(&mut self, id: PeerId, port: u16) -> Vec<Order>;
    fn query(&mut self) -> Vec<Order>;
}

const MAX_BLOCK_SIZE: u64 = 2 << 14;
const MAX_BYTES_PER_REQUEST: u64 = 1024 * 512;
const MAX_PIECES_PEER: usize = 10;

pub struct NormalStrategy {
    peers: HashMap<PeerId, PeerState>,
    pieces_to_request: HashMap<u64, Vec<(u64, u64)>>,
    next_order_id: usize,
    num_pieces: usize,
    piece_length: u64,
    partial_file: PartialFile
}

impl NormalStrategy {
    pub fn new (metainfo: MetaInfo) -> NormalStrategy {
        let num_pieces = metainfo.info.pieces.len();
        let partial_file = PartialFile::new(&metainfo.info);
        let piece_length = partial_file.piece_length();

        NormalStrategy {
            peers: HashMap::new(),
            pieces_to_request: HashMap::new(),//BitVec::from_elem(num_pieces, true),
            next_order_id: 0,
            partial_file: partial_file,
            num_pieces: num_pieces,
            piece_length: piece_length,
        }
    }

    pub fn request_pieces(&mut self) -> Vec<PeerAction> {
        let mut actions = Vec::new();
        let mut ready_peers = self.peers.values()
            .filter(|peer| !peer.peer_choking && peer.has_handshake)
            .collect::<Vec<_>>();

        // sort by score
        ready_peers.sort_by(|&a, &b| b.score.cmp(&a.score));

        for peer in ready_peers.iter() {
            let id = peer.peer_id;
            let missing = self._get_missing(&peer.file);
            let mut msgs = Vec::new();

            if !peer.am_choking {
                msgs.push(PeerMsg::Unchoke);
            }

            info!("Missing these: {:?}", missing);

            for (piece_index, _) in missing.into_iter().enumerate().filter(|&(_, val)| val){
                let mut pieces = NormalStrategy::piece_request(piece_index as u64, self.piece_length, MAX_BLOCK_SIZE);
                let mut requested = Vec::new();
                for piece in pieces.iter() {
                    match piece {
                        &PeerMsg::Request(_, offset, size) => requested.push((offset as u64, (offset + size) as u64)),
                        _ => ()
                    }
                }
                requested.sort();
                self.pieces_to_request.insert(piece_index as u64, requested);
                msgs.append(&mut pieces);
            }

            let req_actions = PeerAction(id, PeerStreamAction::SendMessages(msgs));
            actions.push(req_actions);
        }
        actions
    }

    fn piece_request(piece_index: u64, piece_len: u64, max_block_size: u64) -> Vec<PeerMsg> {
        let mut msgs = Vec::new();
        let mut bytes_requested = 0;

        if piece_len > max_block_size {
            let mut offset = 0;
            while offset < piece_len {
                if (offset + max_block_size > piece_len) {
                    msgs.push(PeerMsg::Request(piece_index as u32, offset as u32, 
                        max_block_size as u32));
                } else {
                    msgs.push(PeerMsg::Request(piece_index as u32, offset as u32,
                        max_block_size as u32));
                }
                offset += max_block_size;
            }
        } else {
            msgs.push(PeerMsg::Request(piece_index as u32, 0, piece_len as u32));
        }

        msgs
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

    fn _make_order(&mut self, action: PeerAction) -> Order {
        self.next_order_id += 1;
        Order {order_id: self.next_order_id, action: action}
    }

    fn _make_msg_order(&mut self, id: PeerId, msgs: Vec<PeerMsg>) -> Order {
        self.next_order_id += 1;
        let action = PeerAction(id, PeerStreamAction::SendMessages(msgs));
        Order {order_id: self.next_order_id, action: action}
    }

    fn _get_missing(&self, peer_file: &PeerFile) -> BitVec { 
        let mut missing = self.partial_file.bit_array();
        missing.negate();
        let mut them = peer_file.bit_array();
        missing.intersect(&them);
        missing
    }
}

const HAVE_SCORE: u64 = 1;
const BITFIELD_SCORE: u64 = 10;
const REQUEST_SCORE: u64 = 1;
const PIECE_SCORE: u64 = 5;

impl Strategy for NormalStrategy {
    fn on_handshake(&mut self, id: PeerId) -> Vec<Order> {
        let peer = PeerState::new(self.num_pieces, id);
        self.peers.insert(id, peer);
        vec![]

    }
    fn on_choke(&mut self, id: PeerId) -> Vec<Order> {
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => peer.peer_choking = true,
            None => {} 
        };
        vec![]
    }
    fn on_unchoke(&mut self, id: PeerId) -> Vec<Order> {
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => peer.peer_choking = false,
            None => {} 
        };
        vec![]

    }
    fn on_interested(&mut self, id: PeerId) -> Vec<Order>  {
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => peer.peer_interested = true,
            None => {} 
        };
        vec![]

    }
    fn on_not_interested(&mut self, id: PeerId) -> Vec<Order> {
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => peer.peer_interested = false,
            None => {} 
        };
        vec![]
    }

    fn on_have(&mut self, id: PeerId, piece_index: usize) -> Vec<Order> {
        let msgs = {
            match self.peers.get_mut(&id) {
                Some(ref mut peer) => {
                    peer.score += HAVE_SCORE;
                    peer.file.set(piece_index as usize, true);
                    let file_vec = peer.file.bit_array();
                    if let Some(true) = file_vec.get(piece_index) {
                        let p_msgs = NormalStrategy::piece_request(piece_index as u64, self.piece_length, MAX_BLOCK_SIZE);
                        p_msgs
                    } else {
                        return Vec::new();
                    }
                }
                None => return Vec::new()
            }
        };
        let p_orders = self._make_msg_order(id, msgs);
        return vec![p_orders];
    }

    fn on_bitfield(&mut self, id: PeerId, bitfield: BitVec) ->  Vec<Order> {
        let mut orders = vec![];
        orders.push(self._make_msg_order(id, vec![PeerMsg::Choke]));

        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                    peer.score += BITFIELD_SCORE;
                    let limit = peer.file.bit_array().len();
                    for (i, bit) in bitfield.iter().enumerate()  {
                        if i >= limit {
                            break;
                        }
                        peer.file.set(i, bit);
                    }
                    return orders;
            },
            None => {} 
        };
        vec![] 
    }

    fn on_request(&mut self, id: PeerId, index: u32, begin: u32, length: u32) -> Vec<Order> {
        let piece_result = self._get_piece_from_req(index as usize, begin, length);
        let order = match piece_result {
            Some(r) => {
                self._make_msg_order(id, vec![r])
            }
            _ => return Vec::new(),
        };

        
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                peer.score += REQUEST_SCORE;
                return vec![order];
            }
            _ => vec![]
        }
    }

    fn on_piece(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order> {
        let block_len = block.len();
        self.partial_file.add_piece(index as usize, begin as usize, block);

        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                peer.score += PIECE_SCORE;
            }
            _ => {}
        }

        
        let blocks = match self.pieces_to_request.remove(&(index as u64)) {
            Some(blocks) => blocks.clone().into_iter().filter(|&(start, end)|  
                start == (index as u64) && end == (index + block_len as u32) as u64).collect(),
            None => vec![]
        };

        if blocks.len() > 0 {
            self.pieces_to_request.insert(index as u64, blocks);
        }
        
        Vec::new()
    }

    fn on_cancel(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order> {
        Vec::new()
    }

    fn on_port(&mut self, id: PeerId, port: u16) -> Vec<Order> {
        Vec::new()
    }

    fn query(&mut self) -> Vec<Order> {
        self.request_pieces().into_iter().map(|a| self._make_order(a)).collect()
    }
}

struct OrderInfo;
type OrderId = usize;

pub struct Order {
    order_id: OrderId,
    pub action: PeerAction
}

impl Order {
    pub fn id(&self) -> OrderId {
        self.order_id
    }
}


