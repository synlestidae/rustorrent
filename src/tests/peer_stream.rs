use wire::{PeerStream, PeerMsg};
use bit_vec::BitVec;
use init;

#[test]
fn test_parses_bitfield_have() {

    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 2, 5, 0]); //Bitfield all zeros
    stream.write_in(vec![0, 0, 0, 5, 4, 0, 0, 0, 0]); //Have(0)
    assert_eq!(stream.message(), Some(PeerMsg::Bitfield(BitVec::from_elem(8, false))));
    assert_eq!(stream.message(), Some(PeerMsg::Have(0)));

}
