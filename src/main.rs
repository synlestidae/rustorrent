extern crate rustorrent;

use rustorrent::bencode::decode;

fn main() {
    let bencoded_text = "8:rustlang".as_bytes().to_vec();
    let decoded_text = decode::bstring_decode(bencoded_text).unwrap();
    println!("Decoded as `{}`", decoded_text);
}
