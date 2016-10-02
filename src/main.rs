extern crate rustorrent;
extern crate sha1;
extern crate mio;

use rustorrent::bencode::decode::{belement_decode, DecodeResult};
use rustorrent::bencode::{BDict};
use rustorrent::metainfo::{MetaInfo, SHA1Hash20b};
use rustorrent::wire::{Protocol, ChanMsg};
use rustorrent::convert::TryFrom;
use rustorrent::bencode::Bencode;

use std::env;
use std::fs::File;
use std::io;
use std::thread::{sleep, spawn};
use std::io::Read;
use mio::channel::{Sender, Receiver};

use sha1::Sha1;


pub fn main() {
    let mut args = env::args();
    if let Some(path_string) = args.nth(1) {
        _begin_with_path(path_string);
    } else {
        _usage();
    }
}

type SuccessType = ();

fn _begin_with_path(path_string: String) -> io::Result<SuccessType> {
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
        return Ok(());
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
        //return parse_result.map(|_| ());
        return Ok(());
    }

    if let Ok(metainfo) = MetaInfo::try_from(bdict) {
        let mut hash_array  = Vec::new();
        for &b in hash.iter() {
            hash_array.push(b);
        }
        _begin_protocol_session(&metainfo, hash_array);
    } 

    return Ok(());
}


fn _begin_protocol_session(info: &MetaInfo, hash: SHA1Hash20b) {
    match Protocol::new(info, hash) {
        (protocol, sender, receiver) => {
            _start_peer_thread(protocol);
            _start_comm_session(sender, receiver);
        }
    }
}

fn _start_peer_thread(protocol: Protocol) {
}

fn _start_comm_session(sender: Sender<ChanMsg>, recv: Receiver<ChanMsg>) {
}

fn _usage() {
    match env::current_exe() {
        Ok(path) => println!("Usage: {} torrent_file", path.display()),
        _ => println!("Invalid arguments. Format is: torrent_file")
    }
}
