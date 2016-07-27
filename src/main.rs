extern crate rustorrent;

use rustorrent::bencode::decode;

fn main() {
    let bencoded_text = "i2015000000e".as_bytes().to_vec();
    let decoded_text = decode::bint_decode(bencoded_text).unwrap();
    println!("Decoded as `{}`", decoded_text);
}
