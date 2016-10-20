use wire::msg::PeerMsg;
use wire::action::{PeerId, PeerAction};
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;

pub trait ServerHandler {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self;
    fn on_peer_connect(&mut self, PeerId) -> PeerAction;
    fn on_message_receive(&mut self, id: PeerId, msg: PeerMsg) -> PeerAction;
    fn on_peer_disconnect(&mut self, id: PeerId) -> PeerAction;
    fn on_loop(&mut self) -> Vec<PeerAction>;
}
