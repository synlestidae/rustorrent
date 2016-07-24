use bencode::{Bencode, BString, BInt, BList, BDict, DecodeError, DecodeErrorKind};

pub fn bstring_decode(bytes: Vec<u8>) -> Result<BString, DecodeError> {
    let bytestring = try!(String::from_utf8(bytes.clone()));
    let mut length_str = String::new();
    if bytestring.contains(':') {
        for ch in &bytes {
            if (*ch as char) != ':' {
                length_str.push((*ch as char));
            } else { break; }
        }
    } else {
        return Err(DecodeError { location: None, kind: DecodeErrorKind::ExpectedByte(':') } )
    }
    let length_int = try!(length_str.parse::<usize>());
    let string_data = &bytes[length_str.len()+1...(length_int+length_str.len())];
    let bstring = BString::new(string_data);
    Ok(bstring)
}

