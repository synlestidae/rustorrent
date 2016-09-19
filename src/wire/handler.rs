use wire::data::PeerMsg;
use file::PartialFile;
use metainfo::MetaInfo;
use convert::TryInto;
use sha1::{Sha1, Digest};
use metainfo::SHA1Hash20b;

pub trait PeerHandler {
    fn handshake(&mut self, info_hash: &SHA1Hash20b, pf: &PartialFile) -> Vec<PeerMsg>;
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
    fn handshake(&mut self, hash: &SHA1Hash20b, pf: &PartialFile) -> Vec<PeerMsg> {
        /*let sha1 = Sha1::new();
        let hash = sha1.update(info).digest().bytes();*/
        let mut id = self.0.to_string().into_bytes();
        id.resize(20, 0);
        vec![PeerMsg::HandShake(_protocol_id.to_string(), hash.clone(), id)]
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
