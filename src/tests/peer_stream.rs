use wire::{PeerStream, PeerMsg};
use bit_vec::BitVec;
use init;

#[test]
fn test_parses_bitfield_have() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 2, 5, 0]); //Bitfield all zeros
    assert_eq!(stream.message(), Some(PeerMsg::Bitfield(BitVec::from_elem(8, false))));
    stream.write_in(vec![0, 0, 0, 5, 4, 0, 0, 0, 0]); //Have(0)
    assert_eq!(stream.message(), Some(PeerMsg::Have(0)));
    assert_eq!(stream.len_in(), 0);
}

#[test]
fn test_parses_unchoke_bitfield_have_() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 1]);
    assert_eq!(stream.message(), Some(PeerMsg::Unchoke));
    stream.write_in(vec![0, 0, 0, 3, 5, 0, 0]); //Bitfield all zeros
    assert_eq!(stream.message(), Some(PeerMsg::Bitfield(BitVec::from_elem(16, false))));
    stream.write_in(vec![0, 0, 0, 5, 4, 0, 0, 0, 0]); //Have(0)
    assert_eq!(stream.message(), Some(PeerMsg::Have(0)));
    assert_eq!(stream.len_in(), 0);
}

#[test]
fn test_parses_choke() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 0]);
    assert_eq!(stream.message(), Some(PeerMsg::Choke));
}

#[test]
fn test_parses_unchoke() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 1]);
    assert_eq!(stream.message(), Some(PeerMsg::Unchoke));
}

#[test]
fn test_parses_interested() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 2]);
    assert_eq!(stream.message(), Some(PeerMsg::Interested));
}

#[test]
fn test_parses_notinterested() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 3]);
    assert_eq!(stream.message(), Some(PeerMsg::NotInterested));
}
