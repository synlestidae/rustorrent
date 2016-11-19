use wire::msg::PeerMsg;
use wire::action::{PeerId, PeerAction};
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;
use wire::peer_info::PeerState;

pub trait ServerHandler {
    fn new(metainfo: MetaInfo, hash: SHA1Hash20b, our_peer_id: &str) -> Self;
    fn on_peer_connect(&mut self, peer: &mut PeerState);
    fn on_message_receive(&mut self, peer: &mut PeerState, msg: PeerMsg);
    fn on_peer_disconnect(&mut self, peer: &mut PeerState);
    fn on_loop(&mut self);
}
