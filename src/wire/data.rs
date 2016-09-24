use std::convert::Into;
use convert::TryFrom;
use std::string::ToString;
use byteorder::{WriteBytesExt, ByteOrder, BigEndian};
use metainfo::SHA1Hash20b;
use file::PartialFile;
use bit_vec::BitVec;

pub enum PeerMsg {
    // info hash, peer id
    HandShake(String, SHA1Hash20b, SHA1Hash20b),
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(BitVec),
    Request(u32, u32, u32),
    Piece(u32, u32, Vec<u8>),
    Cancel(u32, u32, u32),
    Port(u32),
}

impl PeerMsg {
    pub fn id(&self) -> Option<u8> {
        if let &PeerMsg::KeepAlive = self {
            return None;
        } else if let &PeerMsg::HandShake(..) = self {
            return None;
        }

        Some(match self {
            &PeerMsg::KeepAlive => 0, //unreachable
            &PeerMsg::HandShake(..) => 0, //unreachable
            &PeerMsg::Choke => 0,
            &PeerMsg::Unchoke => 1,
            &PeerMsg::Interested => 2,
            &PeerMsg::NotInterested => 3,
            &PeerMsg::Have(_) => 4,
            &PeerMsg::Bitfield(_) => 5,
            &PeerMsg::Request(_, _, _) => 6,
            &PeerMsg::Piece(_, _, _) => 7,
            &PeerMsg::Cancel(_, _, _) => 8,
            &PeerMsg::Port(_) => 9,
        })
    }

    pub fn handshake(protocol_id: String, peer_id: String, hash: &SHA1Hash20b) -> PeerMsg {
        let mut id = peer_id.into_bytes();
        id.resize(20, 0);
        PeerMsg::HandShake(protocol_id, hash.clone(), id)
    }
}

impl Into<Vec<u8>> for PeerMsg {
    fn into(self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut bytes = match self {
            PeerMsg::HandShake(mut protocol_id, mut info_hash, mut peer_id) => {
                let protocol_bytes = protocol_id.into_bytes();
                let mut p_bytes = &protocol_bytes[0..protocol_bytes.len()];
                if p_bytes.len() > 255 {
                    p_bytes = &p_bytes[0..255]
                }
                out.push(p_bytes.len() as u8);
                out.extend_from_slice(p_bytes);
                out.append(&mut vec![0, 0, 0, 0, 0, 0, 0, 0]);
                out.append(&mut info_hash);
                out.append(&mut peer_id);
                return out;
            }
            PeerMsg::Have(piece_index) => {
                out.write_u32::<BigEndian>(piece_index);
                out
            }
            PeerMsg::Bitfield(bit_field) => unimplemented!(),
            PeerMsg::Request(index, begin, length) => {
                out.write_u32::<BigEndian>(index);
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(length);
                out
            }
            PeerMsg::Piece(index, begin, ref block) => {
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(begin);
                out.append(&mut block.clone());
                out
            }
            PeerMsg::Cancel(index, begin, length) => {
                out.write_u32::<BigEndian>(index);
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(length);
                out
            }
            PeerMsg::Port(port) => {
                out.write_u32::<BigEndian>(port);
                out
            }
            _ => out,
        };
        let mut front_part = Vec::new();
        let length = bytes.len() as u32;
        front_part.write_u32::<BigEndian>(length);
        if let Some(id) = self.id() {
            front_part.push(id);
        }
        front_part.append(&mut bytes);
        front_part
    }
}

impl TryFrom<Vec<u8>> for PeerMsg {
    type Err = ();
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Err> {
        unimplemented!();
    }
}
