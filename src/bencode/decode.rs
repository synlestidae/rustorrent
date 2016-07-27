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

pub fn bint_decode(bytes: Vec<u8>) -> Result<BInt, DecodeError> {
    let mut number_string = String::new();
    if bytes.len() > 1 {
        if (bytes[0] as char) == 'i' {
            match bytes[1] as char {
                '0' => {
                    if bytes.len() >= 2 && (bytes[2] as char) == 'e' {
                        return Ok(BInt::new(0u64))
                    } else {
                        return Err(DecodeError { location: None, kind: DecodeErrorKind::ExpectedByte('e') } )
                    }
                }
                _ => {
                    for i in &bytes[1..bytes.len()] {
                        if (*i as char) != 'e' {
                            number_string.push(*i as char);
                        } else { break; }
                    }
                }
            }
        } else {
            return Err(DecodeError { location: None, kind: DecodeErrorKind::ExpectedByte('i') } )
        }
    } else {
        return Err(DecodeError { location: None, kind: DecodeErrorKind::EndOfStream } )
    }
    let parsint = try!(number_string.parse::<u64>());
    let number = BInt::new(parsint);
    Ok(number)
}

