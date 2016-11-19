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
use metainfo::SHA1Hash20b;

pub trait Strategy {
    fn on_handshake(&mut self, peer: &mut PeerState, info_hash: SHA1Hash20b, peer_id: SHA1Hash20b) ;
    fn on_choke(&mut self, peer: &mut PeerState) ;
    fn on_unchoke(&mut self, peer: &mut PeerState) ;
    fn on_interested(&mut self, peer: &mut PeerState) ;
    fn on_not_interested(&mut self, peer: &mut PeerState) ;
    fn on_have(&mut self, peer: &mut PeerState, piece_index: usize) ;
    fn on_bitfield(&mut self, peer: &mut PeerState, bitfield: BitVec) ;
    fn on_request(&mut self, peer: &mut PeerState, index: u32, begin: u32, length: u32) ;
    fn on_piece(&mut self, peer: &mut PeerState, index: u32, begin: u32, block: Vec<u8>) ;
    fn on_cancel(&mut self, peer: &mut PeerState, index: u32, begin: u32, block: Vec<u8>) ;
    fn on_port(&mut self, peer: &mut PeerState, port: u16) ;
    fn query(&mut self, peers: HashMap<PeerId, PeerState>);
}

pub struct BitTorrentProtocol {
    num_pieces: usize,
    piece_length: u64,
    partial_file: PartialFile,
}

impl BitTorrentProtocol {
    pub fn new(metainfo: MetaInfo) -> BitTorrentProtocol {
        let num_pieces = metainfo.info.pieces.len();
        let partial_file = PartialFile::new(&metainfo.info);
        let piece_length = partial_file.piece_length();

        BitTorrentProtocol {
            partial_file: partial_file,
            num_pieces: num_pieces,
            piece_length: piece_length,
        }
    }

    fn _get_piece_from_req(&mut self, index: usize, begin: u32, offset: u32) -> Option<Vec<u8>> {
        if self.partial_file.has_piece(index as usize) {
            let piece = self.partial_file.get_piece_mut(index as usize);
            match piece.get_offset(begin as usize, offset as usize) {
                Some(bytes) => return Some(Vec::from(bytes)),
                _ => {}
            }
        }

        None
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

impl Strategy for BitTorrentProtocol {
    fn on_handshake(&mut self, peer: &mut PeerState, their_hash: SHA1Hash20b, peer_id: SHA1Hash20b)  {
        if /*their_hash == &self.hash*/ true {
            peer.has_handshake = true;
            peer.choke(false);
            peer.interested(true);
        } else {
            peer.disconnect();
        }
    }

    fn on_choke(&mut self, peer: &mut PeerState) {
    }

    fn on_unchoke(&mut self, peer: &mut PeerState) {
    }

    fn on_interested(&mut self, peer: &mut PeerState) {
    }

    fn on_not_interested(&mut self, peer: &mut PeerState) {
    }

    fn on_have(&mut self, peer: &mut PeerState, piece_index: usize) {
        peer.score += HAVE_SCORE;
    }

    fn on_bitfield(&mut self, peer: &mut PeerState, bitfield: BitVec) {
        peer.score += BITFIELD_SCORE;
    }

    fn on_request(&mut self, peer: &mut PeerState, index: u32, begin: u32, length: u32)  {
        match self._get_piece_from_req(index as usize, begin, length) {
            Some(data) => {
                peer.send_piece_data(index, begin, data);
            } 
            _ => return
        }
        
        peer.score += REQUEST_SCORE;
    }

    fn on_piece(&mut self, peer: &mut PeerState, index: u32, begin: u32, block: Vec<u8>)  {
        let block_len = block.len();
        self.partial_file.add_piece(index as usize, begin as usize, block);
        peer.score += PIECE_SCORE;
    }

    fn on_cancel(&mut self, peer: &mut PeerState, index: u32, begin: u32, block: Vec<u8>)  {
    }

    fn on_port(&mut self, peer: &mut PeerState, port: u16)  {
    }

    fn query(&mut self, peers: HashMap<PeerId, PeerState>) {
    }
}