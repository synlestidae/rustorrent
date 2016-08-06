extern crate rustorrent;

use rustorrent::bencode::decode;

fn main() {
    let bencoded_text = "l4:spam4:eggse".as_bytes().to_vec();
    let decoded_list = decode::blist_decode(&bencoded_text, &mut 0).unwrap();
    println!("Decoded as: {:?}", decoded_list);
}
