use std::convert::Into;
use std::string::ToString;
use byteorder::{WriteBytesExt, ByteOrder, BigEndian};

pub enum PeerMsg {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<bool>),
    Request(u32, u32, u32),
    Piece(u32, u32, Vec<u8>),
    Cancel(u32, u32, u32),
    Port(u32)
}

impl PeerMsg {
    pub fn id(&self) -> Option<u8> {
        if let &PeerMsg::KeepAlive = self {
            return None;
        }

        Some(match self {
            &PeerMsg::KeepAlive => 0, //unreachable
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
}

impl Into<Vec<u8>> for PeerMsg {
    fn into(self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut bytes = match self {
           PeerMsg::Have(piece_index) => {
                out.write_u32::<BigEndian>(piece_index);
                out
            },
            PeerMsg::Bitfield(bit_field) => unimplemented!(),
            PeerMsg::Request(index, begin, length) => {
                out.write_u32::<BigEndian>(index);
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(length);
                out
            },
            PeerMsg::Piece(index, begin, ref block) => {
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(begin);
                out.append(&mut block.clone());
                out
            },
            PeerMsg::Cancel(index, begin, length) => {
                out.write_u32::<BigEndian>(index);
                out.write_u32::<BigEndian>(begin);
                out.write_u32::<BigEndian>(length);
                out
            },
            PeerMsg::Port(port) => {
                out.write_u32::<BigEndian>(port);
                out
            },
            _ => out
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
