use wire::{PeerStream, PeerMsg};
use std::iter::FromIterator;
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
    assert_eq!(stream.message(), None);
    assert_eq!(stream.len_in(), 0);
}

#[test]
fn test_parses_handshake() {
    let mut stream = PeerStream::new(0);
    let hash = [0; 20].into_iter().map(|&x| x).collect();
    let handshake = PeerMsg::HandShake("BitTorrent protocol".to_string(), 
         hash,
        "rust-torrent-1234567".to_string().into_bytes()); 
    stream.write_in(handshake.clone().into());
    assert_eq!(Some(handshake), stream.message());
    assert_eq!(stream.len_in(), 0);
}


#[test]
fn test_parses_keepalive() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 0]);
    assert_eq!(stream.message(), Some(PeerMsg::KeepAlive));
    assert_eq!(stream.len_in(), 0);
    assert_eq!(stream.message(), None);
}

#[test]
fn test_parses_choke() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 0]);
    assert_eq!(stream.message(), Some(PeerMsg::Choke));
    assert_eq!(stream.len_in(), 0);
    assert_eq!(stream.message(), None);
}

#[test]
fn test_parses_unchoke() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 1]);
    assert_eq!(stream.message(), Some(PeerMsg::Unchoke));
    assert_eq!(stream.len_in(), 0);
    assert_eq!(stream.message(), None);
}

#[test]
fn test_parses_interested() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 2]);
    assert_eq!(stream.message(), Some(PeerMsg::Interested));
    assert_eq!(stream.len_in(), 0);
    assert_eq!(stream.message(), None);
}

#[test]
fn test_parses_notinterested() {
    let mut stream = PeerStream::new(0);
    stream.write_in(vec![0, 0, 0, 1, 3]);
    assert_eq!(stream.message(), Some(PeerMsg::NotInterested));
    assert_eq!(stream.len_in(), 0);
    assert_eq!(stream.message(), None);
}
