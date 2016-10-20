use std::convert::Into;
use convert::TryFrom;
use std::string::ToString;
use byteorder::{WriteBytesExt, ByteOrder, BigEndian};
use metainfo::SHA1Hash20b;
use file::PartialFile;
use bit_vec::BitVec;
use std::str;

#[derive(Debug)]
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

#[derive(Debug)]
pub enum MsgParseError {
    TooShort,
    TooShortForId,
    InvalidId,
    Malformed(&'static str),
    UnknownProtocol,
}

pub fn parse_peermsg(bytes: &[u8]) -> Result<(PeerMsg, usize), MsgParseError> {
    const LEN_LEN: usize = 4;
    const INT_LEN: usize = 4;
    const ID_LEN: usize = 1;
    const PORT_LEN: usize = 2;

    info!("Bytes: len {:?}",
          if bytes.len() > 80 {
              &bytes[0..80]
          } else {
              bytes
          });

    if bytes.len() < 4 {
        return Err(MsgParseError::TooShort);
    }

    let mut len = BigEndian::read_u32(&bytes[0..4]) as usize;
    info!("LENNY {}", len);

    if len == 0 {
        return Ok((PeerMsg::KeepAlive, 4));
    } else if len == 323119476 {
        return parse_handshake(bytes);
    } else if len + 4 > bytes.len() {
        info!("Len is {} but need {}", bytes.len(), len);
        return Err(MsgParseError::TooShort);
    } else if len == 323119476 {
        return parse_handshake(bytes);
    }

    let bytes = &bytes[4..len];
    len = len - 4;
    if bytes.len() < len {
        info!("Len is {} but need {}", bytes.len(), len);
        return Err(MsgParseError::TooShort);
    }

    info!("Message has id {}", bytes[0]);

    let result = match bytes[0] {
        0 => Ok(PeerMsg::Choke),
        1 => Ok(PeerMsg::Unchoke),
        2 => Ok(PeerMsg::Interested),
        3 => Ok(PeerMsg::NotInterested),
        4 => {
            if len < ID_LEN + INT_LEN {
                return Err(MsgParseError::TooShortForId);
            }
            let piece_index = BigEndian::read_u32(&bytes[1..(1 + INT_LEN)]);
            Ok(PeerMsg::Have(piece_index))
        }
        5 => {
            let bitfield_bytes = &bytes[0..len - 4];
            Ok(PeerMsg::Bitfield(BitVec::from_bytes(bitfield_bytes)))
        }
        6 => {
            if len != 13 {
                return Err(MsgParseError::TooShortForId);
            }

            match _parse_three_u32(&bytes[1..1 + (INT_LEN * 3)]) {
                (index, begin, end) => Ok((PeerMsg::Request(index, begin, end))),
            }
        }
        7 => {
            if len <= 9 {
                return Err(MsgParseError::TooShortForId);
            }
            let index = BigEndian::read_u32(&bytes[1..(1 + INT_LEN)]);
            let begin = BigEndian::read_u32(&bytes[(1 + INT_LEN)..(1 + INT_LEN * 2)]);
            let block = &bytes[(1 + INT_LEN * 2)..len];
            let block_data = Vec::from(block);
            Ok((PeerMsg::Piece(index, begin, block_data)))
        }
        8 => {
            if len != 13 {
                return Err(MsgParseError::TooShortForId);
            }

            match _parse_three_u32(&bytes[1..1 + (INT_LEN * 3)]) {
                (index, begin, end) => Ok((PeerMsg::Cancel(index, begin, end))),
            }
        }
        9 => {
            if len != 3 {
                return Err(MsgParseError::TooShortForId);
            }
            let port = BigEndian::read_u32(&bytes[1..(1 + PORT_LEN)]);
            Ok(PeerMsg::Port(port))
        }
        _ => Err(MsgParseError::InvalidId),
    };

    result.map(|msg| (msg, len + 4))
}

fn _parse_three_u32(bytes: &[u8]) -> (u32, u32, u32) {
    let index = BigEndian::read_u32(&bytes[0..4]);
    let begin = BigEndian::read_u32(&bytes[4..8]);
    let end = BigEndian::read_u32(&bytes[8..12]);

    (index, begin, end)
}

pub fn parse_handshake(bytes: &[u8]) -> Result<(PeerMsg, usize), MsgParseError> {
    const BITTORRENT_PROTOCOL: &'static str = "BitTorrent protocol";

    if bytes.len() < 1 + 19 + 8 + 20 + 20 {
        // length byte, BitTorrent protocol, reserved , hash, peer id (unknown)
        return Err(MsgParseError::TooShort);
    }

    if !(bytes[0] == 19 || bytes[0] == 323119476) {
        info!("Bad bytes {}", bytes[0]);
        return Err(MsgParseError::Malformed("Expected handshake to have protocol ID of 19 bytes"));
    }
    match str::from_utf8(&bytes[1..(1 + 19)]) {
        Ok(BITTORRENT_PROTOCOL) => (),
        _ => return Err(MsgParseError::UnknownProtocol),
    }

    Ok((PeerMsg::HandShake(BITTORRENT_PROTOCOL.to_string(),
                           Vec::from(&bytes[(1 + 19 + 8)..(1 + 19 + 8 + 20)]),
                           Vec::from(&bytes[(1 + 19 + 8 + 20)..(1 + 19 + 8 + 20 + 20)])),
        (1 + 19 + 8 + 20 + 20)))
}

impl TryFrom<Vec<u8>> for PeerMsg {
    type Err = ();
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Err> {
        unimplemented!();
    }
}
