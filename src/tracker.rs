use std::net::Ipv6Addr;
use convert::TryFrom;
use metainfo::SHA1Hash20b;
use bencode::{BDict, BString, DecodeError, DecodeErrorKind};

pub struct TrackerReq {
    pub info_hash: SHA1Hash20b,
    pub peer_id: SHA1Hash20b,
    pub port: u32,
    pub uploaded: u64,
    pub left: u64,
    pub compact: bool,
    pub no_peer_id: bool,
    pub event: TrackerEvent,
    pub ip: Option<Ipv6Addr>,
    pub numwant: Option<u32>,
    pub key: Option<String>,
    pub trackerid: Option<String>
}

fn missing_field(fld: &str) -> DecodeError {
    DecodeError {
        position: None,
        kind: DecodeErrorKind::MissingField(fld.to_string())
    }
}

impl TryFrom<BDict> for TrackerReq {
    type Err = DecodeError;
    fn try_from(dict: BDict) -> Result<Self, Self::Err> {
        /*let info_hash: BString = try!(dict.get_copy("info hash").ok_or(missing_field("info_hash")));
        let peer_id: BString = try!(dict.get_copy("peer id").ok_or(missing_field("peer id")));
        let port: BInt = try!(dict.get_copy("port").ok_or(missing_field("port")));
        let uploaded: BInt = try!(dict.get_copy("uploaded").ok_or(missing_field("uploaded")));
        let left: BInt = try!(dict.get_copy("uploaded").ok_or(missing_field("uploaded")));*/
        unimplemented!();
    }
}

impl TryFrom<BDict> for TrackerResp {
    type Err = DecodeError;
    fn try_from(dict: BDict) -> Result<Self, Self::Err> {
        let failure_reason: Option<BString> = dict.get_copy("failure reason");
        let warning_message: Option<BString> = dict.get_copy("warning message");
        unimplemented!();
    }
}

pub enum TrackerEvent {
    Started,
    Stopped, 
    Completed
}

pub struct TrackerResp {
    pub failure_reason: Option<String>,
    pub warning_reason: Option<String>,
    pub interval: u32,
    pub min_interval: u32,
    pub tracker_id: String,
    pub complete: u32,
    pub peers: Vec<Peer>
}

pub struct Peer {
    pub peer_id: String,
    pub ip: Ipv6Addr,
    pub port: u32
}
