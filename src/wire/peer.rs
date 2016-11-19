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
use wire::strategy::{Strategy, BitTorrentProtocol};

const TIMEOUT_SECONDS: u64 = 60 * 5;
const KEEPALIVE_PERIOD: u64 = 30;

pub struct PeerServer {
    hash: SHA1Hash20b,
    our_peer_id: String,
    partial_file: PartialFile,
    num_pieces: usize,
    pieces_to_request: BitVec,
    strategy: BitTorrentProtocol,
}

const PROTOCOL_ID: &'static str = "BitTorrent protocol";

impl ServerHandler for PeerServer {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self {
        let num_pieces = metainfo.info.pieces.len();
        let partial_file = PartialFile::new(&metainfo.info);
        let pl = partial_file.piece_length();

        PeerServer {
            hash: hash,
            our_peer_id: our_peer_id.to_string(),
            partial_file: partial_file,
            num_pieces: num_pieces,
            pieces_to_request: BitVec::from_elem(num_pieces, true),
            strategy: BitTorrentProtocol::new(metainfo),
        }
    }

    fn on_peer_connect(&mut self, peer: &mut PeerState) {
        let handshake = PeerMsg::handshake(PROTOCOL_ID.to_string(),
                                           self.our_peer_id.to_string(),
                                           &self.hash);

        peer.write_message_out(handshake.into());
    }

    fn on_message_receive(&mut self, peer: &mut PeerState, msg: PeerMsg) {
        self._on_message_receive(peer, msg);
    }

    fn on_peer_disconnect(&mut self, peer: &mut PeerState) {
        //meh 
    }

    // remove peers that have not replied in five minutes
    fn on_loop(&mut self) {
    } 
}

impl PeerServer {
    /*fn _remove_old_peers(&mut self) {
        let for_removal = self._get_timeout_ids();
        for id in for_removal {
            self.peers.remove(&id);
        }
    }*/

    /*fn _get_timeout_ids(&self) -> Vec<PeerId> {
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
    }*/

    /*fn _get_piece_from_req(&mut self, index: usize, begin: u32, offset: u32) -> Option<PeerMsg> {
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

    }*/

    fn _on_message_receive(&mut self, peer: &mut PeerState, msg: PeerMsg) {
        peer.last_msg_time = SystemTime::now();

        if !peer.has_handshake {
            match msg {
                PeerMsg::HandShake(_, ref their_hash, ref peer_id) => {
                    self.strategy.on_handshake(peer, their_hash.clone(), peer_id.clone());
                    return;
                },
                _ => {
                    peer.disconnect();
                    return;
                }
            }
        }

        // handshake is okay
        let orders = match msg {
            PeerMsg::HandShake(_, hash, id) => self.strategy.on_handshake(peer, hash, id),
            PeerMsg::KeepAlive => return,
            PeerMsg::Choke => self.strategy.on_choke(peer),
            PeerMsg::Unchoke => self.strategy.on_unchoke(peer),
            PeerMsg::Interested => self.strategy.on_interested(peer),
            PeerMsg::NotInterested => self.strategy.on_not_interested(peer),
            PeerMsg::Have(pi) => self.strategy.on_have(peer, pi as usize),
            PeerMsg::Bitfield(bit_vec) => self.strategy.on_bitfield(peer, bit_vec),
            PeerMsg::Request(index, begin, length) => {
                self.strategy.on_request(peer, index, begin, length)
            }
            PeerMsg::Piece(index, begin, block) => self.strategy.on_piece(peer, index, begin, block),
            PeerMsg::Cancel(index, begin, block) => return,
            // self.strategy.on_cancel(id, index, begin, block),
            PeerMsg::Port(port) => self.strategy.on_port(peer, port as u16),
        };
    }
}

