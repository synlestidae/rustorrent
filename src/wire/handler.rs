pub trait PeerHandler {
    fn handshake(&self) -> bool;
    fn on_message_receive(&self, msg: PeerMsg);
    fn peer_choking(&self) -> bool;
    fn peer_interested(&self) -> bool;
}
