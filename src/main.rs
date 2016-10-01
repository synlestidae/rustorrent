use rustorrent::bencode::decode;
use std::env;
use std::fs::File;
use std::io;
use std::thread::{sleep, spawn};

pub fn main() {
    let args = env::args();
    if let Some(path_string) = args.nth(0) {
        _begin_with_path(path_string);
    } else {
        _usage();
    }
}

fn _begin_with_path(path_string: String) -> io::Result<> {
    match File::open(&path_string) {
        Ok(read) => {
            let mut bytes = Vec::new(); 
            try!(read.read_to_end(&mut bytes));
            match BDict::try_from(bytes) {
                Ok(bdict) => {
                    let sha1 = Sha1::new();
                    sha1.update(&bytes);
                    let hash = sha1.digest().bytes();
                    _begin_session(bdict, hash);
                },
                Err(parse_error) => {
                    println!("Error while parsing bencode dictionary: {}", parse_error); 
                }
            }
        },
        Err(err) => {
            println!("Failed to open file: {}", err)
        }
    }
}

fn _begin_session(dict: &BDict, hash: SHA1Hash20b) {
    match MetaInfo::try_from(dict) {
        Ok(metainfo) => {
            _begin_protocol_session(dict: &BDict, hash: SHA1Hash20b);
        },
        Err(metainfo_error) => println!("Problem with metainfo file: {}", metainfo_error)
    }
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
    loop {
        sleep(
    }
}

fn _usage() {
    match env::current_exe() {
        Ok(path) => println!("Usage: {} torrent_file", path),
        _ => println!("Invalid arguments. Format is: torrent_file")
    }
}
