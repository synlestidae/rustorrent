use wire::data::PeerMsg;
use file::PartialFile;
use metainfo::MetaInfo;
use convert::TryInto;
use sha1::{Sha1, Digest};

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
        /*if let Ok(info) = info.info.try_into() {
            let sha1 = Sha1::new();
            let hash = sha1.update(info).digest().bytes();
            let id = self.id.to_string().into_bytes();
            id.resize(0);
            PeerMsg::HandShake(_protocol_id.to_string().into_bytes(), hash, id)
        } else {
            vec![]
        }*/
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
