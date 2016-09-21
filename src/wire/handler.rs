use wire::data::PeerMsg;
use file::PartialFile;
use metainfo::MetaInfo;
use convert::TryInto;
use sha1::{Sha1, Digest};
use metainfo::SHA1Hash20b;

pub trait PeerHandler {
    fn handshake(&mut self, info: &MetaInfo, pf: &PartialFile) -> Vec<PeerMsg>;
    fn on_message_receive(&mut self, info: &MetaInfo, pf: &PartialFile, msg: PeerMsg) -> Vec<PeerMsg>;
    fn peer_choking(&mut self, info: &MetaInfo, pf: &PartialFile) -> bool;
    fn peer_interested(&self, info: &MetaInfo, pf: &PartialFile) -> bool;
}

pub struct BasicHandler(String);

impl BasicHandler {
    pub fn new(id: String) -> BasicHandler {
        BasicHandler(id)
    }
}

const _protocol_id: &'static str = "rustorrent-beta";

impl PeerHandler for BasicHandler {
    fn handshake(&mut self, info: &MetaInfo, pf: &PartialFile) -> Vec<PeerMsg> {
        unimplemented!();
    }
    fn on_message_receive(&mut self, info: &MetaInfo, pf: &PartialFile, msg: PeerMsg) -> Vec<PeerMsg> {
        unimplemented!()
    }
    fn peer_choking(&mut self, info: &MetaInfo, pf: &PartialFile) -> bool {
        unimplemented!()
    }
    fn peer_interested(&self, info: &MetaInfo, pf: &PartialFile) -> bool {
        unimplemented!()
    }

}

pub type PeerId = u32;
pub struct PeerAction(PeerId, Vec<PeerMsg>);

pub trait ServerHandler {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b) -> Self;
    fn on_peer_connect(&mut self, PeerId) -> PeerAction;
    fn on_message_receive(&mut self, id: PeerId, partial_file: &mut PartialFile) -> PeerAction;
    fn on_peer_disconnect(&mut self, id: PeerId) -> PeerAction;
    fn on_loop(&mut self) -> PeerAction;
}
