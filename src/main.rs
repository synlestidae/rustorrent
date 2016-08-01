extern crate rustorrent;

use rustorrent::bencode::decode;

fn main() {
    let bencoded_text = "4:spam8:rustlang".as_bytes().to_vec();
    println!("{:?}", bencoded_text);
    let (decoded_text, rest) = decode::bstring_decode(bencoded_text).unwrap();
    println!("Decoded as `{}`, remaining: `{:?}`", decoded_text, rest);
}
