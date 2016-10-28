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

pub trait Strategy {
    fn query(&mut self, pending: Vec<&Order>, done: Vec<OrderResult>, peers: Vec<&PeerState>, file: &PartialFile) -> Vec<Order>;
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
}

const MAX_BLOCK_SIZE: u64 = 2 << 14;
const MAX_BYTES_PER_REQUEST: u64 = 1024 * 512;
const MAX_PIECES_PEER: usize = 10;

pub struct NormalStrategy {
    orders: HashMap<OrderId, OrderInfo>,
    peers: HashMap<PeerId, PeerState>,
    pieces_to_request: BitVec,
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
            orders: HashMap::new(),
            peers: HashMap::new(),
            pieces_to_request: BitVec::from_elem(num_pieces, true),
            next_order_id: 0,
            partial_file: partial_file,
            num_pieces: num_pieces,
            piece_length: piece_length,
        }
    }

    pub fn request_pieces<F: PartialFileTrait>(&mut self, peers: Vec<&PeerState>, partial_file: &F) -> Vec<PeerAction> {
        let mut actions = Vec::new();
        let ready_peers = peers.iter().filter(|peer| !peer.peer_choking && !peer.am_choking && peer.has_handshake);
        for peer in ready_peers {
            let id = peer.peer_id;
            let mut missing = partial_file.bit_array();

            missing.negate();
            let mut them = peer.file.bit_array();
            missing.intersect(&them);
            missing.intersect(&self.pieces_to_request);

            //now have eligible pieces to request
            let mut pieces = missing.into_iter().enumerate().filter(|&(_, x)| x).map(|(i, _)| i as u64).collect::<Vec<u64>>();
            let piece_len = self.piece_length;
            let mut bytes_requested = 0;
            let mut pieces_request = 0;

            let mut msgs = Vec::new();
            for (piece_count, piece_index) in pieces.into_iter().enumerate() {
                if bytes_requested > MAX_BYTES_PER_REQUEST || piece_count > MAX_PIECES_PEER {
                    break;
                }
                let mut pieces = NormalStrategy::piece_request(piece_index, piece_len, MAX_BLOCK_SIZE);
                self.pieces_to_request.set(piece_index as usize, false);
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
}

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
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                peer.file.set(piece_index as usize, true);
                if !peer.partial_file.get(piece_index) {
                    let p_msgs = NormalStrategy::piece_request(piece_index, piece_len, MAX_BLOCK_SIZE);
                }
            }
            None => {} 
        };
        vec![]
    }

    fn on_bitfield(&mut self, id: PeerId, bitfield: BitVec) ->  Vec<Order> {
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                    let limit = peer.file.bit_array().len();
                    for (i, bit) in bitfield.iter().enumerate()  {
                        if i >= limit {
                            break;
                        }
                        peer.file.set(i, bit);
                    }
                },
            None => {} 
        };
        vec![]
    }
    fn on_request(&mut self, id: PeerId, index: u32, begin: u32, length: u32) -> Vec<Order> {
        let piece_result = self._get_piece_from_req(index as usize, begin, length);
        match self.peers.get_mut(&id) {
            Some(ref mut peer) => {
                let response = piece_result;
                match response {
                    Some(r) => unimplemented!(),
                    _ => Vec::new(),
                }
            },
            _ => Vec::new()
        }
    }

    fn on_piece(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order> {
        self.partial_file.add_piece(index as usize, begin as usize, block);
        Vec::new()
    }

    fn on_cancel(&mut self, id: PeerId, index: u32, begin: u32, block: Vec<u8>) -> Vec<Order> {
        Vec::new()
    }

    fn on_port(&mut self, id: PeerId, port: u16) -> Vec<Order> {
        Vec::new()
    }

    fn query(&mut self, pending: Vec<&Order>, done: Vec<OrderResult>, peers: Vec<&PeerState>, file: &PartialFile) 
        -> Vec<Order> {
        let actions = self.request_pieces(peers, file);
        let mut orders = Vec::new();

        for action in actions {
            orders.push(make_order(action));
            self.next_order_id += 1;
        }

        orders
    }

    fn _make_order(&mut self, action: PeerStreamAction) {
        Order {order_id: self.next_order_id, action: action, status: OrderStatus::NotStarted}
    }

}


struct OrderInfo;
type OrderId = usize;

pub struct OrderResult {
    status: OrderStatus,
    order_id: OrderId
}

pub struct Order {
    status: OrderStatus,
    order_id: OrderId,
    pub action: PeerAction
}

impl Order {
    pub fn complete(&self, status: OrderStatus) -> OrderResult {
        OrderResult { order_id: self.order_id, status: status}
    }

    pub fn id(&self) -> OrderId {
        self.order_id
    }

    pub fn status(&self) -> OrderStatus {
        self.status
    }

    pub fn set_status(&mut self, status: OrderStatus) {
        self.status = status;
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum OrderStatus {
    NotStarted,
    InProgress, 
    Failed,
    Done
}
