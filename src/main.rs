extern crate rustorrent;
extern crate sha1;
extern crate mio;
extern crate hyper;

use rustorrent::bencode::decode::{belement_decode, DecodeResult};
use rustorrent::bencode::{BDict};
use rustorrent::metainfo::{MetaInfo, SHA1Hash20b};
use rustorrent::wire::{Protocol, ChanMsg, Stats};
use rustorrent::convert::TryFrom;
use rustorrent::bencode::Bencode;
use rustorrent::bencode::DecodeError;
use rustorrent::metainfo::MetaInfoError;
use rustorrent::tracker::HttpTrackerHandler;
use rustorrent::tracker::TrackerEvent;

use std::env;
use std::fs::File;
use std::io;
use std::time::Duration;
use std::thread::{sleep, spawn};
use std::io::Read;
use std::thread;
use std::thread::JoinHandle;

use hyper::Url;
use rustorrent::tracker::http::TrackerHandler;
use rustorrent::tracker::TrackerReq;

use mio::channel::{Sender, Receiver};
use sha1::Sha1;

const DEFAULT_PORT: u32 = 12001;
const DEFAULT_PEER_ID : &'static str = "rustorrent-0.1";


pub fn main() {
    let mut args = env::args();
    if let Some(path_string) = args.nth(1) {
        let result = _begin_with_path(path_string);
    } else {
        _usage();
    }
}

type SuccessType = ();

#[derive(Debug)]
enum FatalError {
    IOError(io::Error),
    DecodeError(DecodeError),
    MetaInfoError(MetaInfoError)
}

fn _begin_with_path(path_string: String) -> Result<SuccessType, FatalError> {
    let mut bytes: Vec<u8> = Vec::new();
    let belement: Bencode;
    let bdict: BDict;
    let metainfo: MetaInfo;

    //read the file and change the result type if fail
    let mut file_open_result = File::open(&path_string);
    if let Ok(mut read) = file_open_result {
        let read_result = read.read_to_end(&mut bytes);
        if !read_result.is_ok() {
            return Ok(());
        }
    } else {
        return Err(FatalError::IOError(file_open_result.err().unwrap()));
    }

    // compute the very important metainfo hash
    let mut sha1 = Sha1::new();
    sha1.update(&bytes);
    let hash = sha1.digest().bytes();

    // parse into a bencoded structure
    let parse_result = belement_decode(&bytes);
    if let Ok(DecodeResult(Bencode::BDict(dict), offset)) = parse_result {
        bdict = dict;
    } else {
        return Err(FatalError::DecodeError(parse_result.err().unwrap()));
    }

    let metainfo_result = MetaInfo::try_from(bdict);
    if let Ok(metainfo) = metainfo_result {
        let mut hash_array  = Vec::new();
        for &b in hash.iter() {
            hash_array.push(b);
        }
        _begin_protocol_session(&metainfo, hash_array);
    } else {
        return Err(FatalError::MetaInfoError(metainfo_result.err().unwrap()));
    }

    return Ok(());
}


fn _begin_protocol_session(info: &MetaInfo, hash: SHA1Hash20b) {
    match Protocol::new(info, hash.clone(), DEFAULT_PEER_ID) {
        (protocol, sender, receiver) => {
            let pwp = _start_peer_wire_protocol_thread(protocol);
            _start_tracker(&hash, info, &DEFAULT_PEER_ID.to_string().into_bytes(), sender, receiver);
        }
    }
}

fn _start_peer_wire_protocol_thread(mut protocol: Protocol) -> JoinHandle<()> {
    thread::spawn(move || protocol.run())
}

fn _start_tracker(hash: &SHA1Hash20b, info: &MetaInfo, peer_id: &SHA1Hash20b, sender: Sender<ChanMsg>, recv: Receiver<ChanMsg>) {
    let SLEEP_DURATION: Duration = Duration::from_millis(10);

    let url_result = Url::parse(&info.announce);
    if !url_result.is_ok() {
        return; //TODO Signal some kind of parse error
    }
    let mut stats = Stats::new();
    let url = url_result.unwrap();
    let mut handler = HttpTrackerHandler::new(url);
    let request: TrackerReq = _get_request_obj(hash, peer_id, info, &stats); 
    sender.send(ChanMsg::StatsRequest);
    let response = match handler.request(&request) {
        Ok(response) => {
            response 
        },
        Err(_) => {
            return; 
        }
    };

    loop {
        match recv.try_recv() {
            Ok(ChanMsg::StatsResponse(new_stats)) => stats = new_stats,
            _ => ()
        }
        thread::sleep(SLEEP_DURATION);
    }

    //TODO Implement this - goal is that it queries that tracker at a defined period
    //Sends list of peers to pwp using Sender
}

fn _get_request_obj(hash: &SHA1Hash20b, peer_id: &SHA1Hash20b, info: &MetaInfo, stats: &Stats) ->  TrackerReq {
    let mut info_hash = Vec::new();
    info_hash.resize(20, 0);

    match info.info.original {
        Some(ref original_dict) => info_hash = original_dict.hash(),
        _ => ()
    };

    TrackerReq {
        info_hash: info_hash,
        peer_id: peer_id.clone(),
        port: DEFAULT_PORT,
        uploaded: 0,
        left: info.info.pieces.len() as u64,
        compact: false,
        no_peer_id: false,
        event: TrackerEvent::Started,
        ip: None,
        numwant: None,
        key: None,
        trackerid: None
    }
}

fn _usage() {
    match env::current_exe() {
        Ok(path) => println!("Usage: {} torrent_file", path.display()),
        _ => println!("Invalid arguments. Format is: torrent_file")
    }
}
