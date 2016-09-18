#[allow(unused_imports)]
use metainfo::MetaInfo;
#[allow(unused_imports)]
use bencode::{Bencode, BDict};
#[allow(unused_imports)]
use bencode::decode::belement_decode;
#[allow(unused_imports)]
use std::io::prelude::*;
#[allow(unused_imports)]
use std::fs::File;
#[allow(unused_imports)]
use convert::TryFrom;

#[test]
pub fn test_parses_torrent_metainfo_file() {
    let mut file = File::open("src/tests/data/ubuntu-gnome-14.04.5-desktop-amd64.torrent").unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes);
    let dict = belement_decode(&bytes).unwrap().0;
    match dict {
        Bencode::BDict(bdict) => {
            MetaInfo::try_from(bdict).unwrap();
        }
        _ => panic!("Got wrong kind of object"),
    }
}

#[test]
pub fn test_parses_torrent_metainfo_file_2() {
    let mut file = File::open("src/tests/data/adventures_holmes_archive.torrent").unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes);
    let decode_result = belement_decode(&bytes).unwrap();
    let dict = decode_result.0;
    match dict {
        Bencode::BDict(bdict) => {
            MetaInfo::try_from(bdict).unwrap();
        }
        _ => panic!("Got wrong kind of object"),
    }
}
