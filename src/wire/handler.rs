use wire::data::PeerMsg;
use file::PartialFile;
use metainfo::MetaInfo;

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

impl PeerHandler for BasicHandler {
    fn handshake(&mut self, info: &MetaInfo, pf: &PartialFile) -> Vec<PeerMsg> {
        unimplemented!()
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
