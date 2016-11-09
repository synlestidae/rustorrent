use wire::msg::PeerMsg;

pub type PeerId = u32;
pub struct PeerAction(pub PeerId, pub PeerStreamAction);
pub enum PeerStreamAction {
    Nothing,
    SendMessages(Vec<PeerMsg>),
    Disconnect,
}
