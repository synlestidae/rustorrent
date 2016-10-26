use wire::peer_info::PeerState;
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
}

pub struct NormalStrategy {
    orders: HashMap<OrderId, OrderInfo>,
    pieces_to_request: BitVec,
    next_order_id: usize
}

impl NormalStrategy {
    pub fn new() -> NormalStrategy {
        NormalStrategy {
            orders: HashMap::new(),
            pieces_to_request: BitVec::new(),
            next_order_id: 0
        }
    }

    fn _request_pieces(&mut self, peers: Vec<&PeerState>, partial_file: &PartialFile) -> Vec<PeerAction> {
        let mut actions = Vec::new();
        for peer in peers {
            let id = peer.peer_id;
            // Skip choking peers
            if peer.peer_choking || peer.am_choking || !peer.has_handshake {
                continue;
            }

            let mut missing = partial_file.bit_array();
            missing.negate();
            let mut them = peer.file.bit_array();
            missing.intersect(&them);
            missing.intersect(&self.pieces_to_request);

            //now have eligible pieces to request
            let mut pieces = missing.into_iter().enumerate().filter(|&(_, x)| x).map(|(i, _)| i as u64).collect::<Vec<u64>>();
            let piece_len = partial_file.piece_length();
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
}

impl Strategy for NormalStrategy {
    fn query(&mut self, pending: Vec<&Order>, done: Vec<OrderResult>, peers: Vec<&PeerState>, file: &PartialFile) 
        -> Vec<Order> {
        let actions = self._request_pieces(peers, file);
        let mut orders = Vec::new();

        for action in actions {
            orders.push(Order {order_id: self.next_order_id, action: action, status: OrderStatus::NotStarted});
            self.next_order_id += 1;
        }

        orders
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
