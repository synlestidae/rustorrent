pub enum PeerMsg {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u64),
    Bitfield(Vec<bool>),
    Request(u64, u64, u64),
    Piece(u64, u64, u64),
    Cancel(u64, u64, u64),
    Port(u32)
}
