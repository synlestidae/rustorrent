use bencode::{Bencode, BString, BInt, BList, BDict, DecodeError, DecodeErrorKind};

pub fn bstring_decode(bytes: Vec<u8>) -> Result<(BString, Vec<u8>), DecodeError> {
    let mut length_str = String::new();
    let mut position = 0u64;
    if bytes.len() > 0 && (bytes[0] as char).is_digit(10) {
        'topl: for ch in &bytes {
            position += 1;
            if (*ch as char) == ':' {
                for byte in &bytes {
                    if (*byte as char) != ':' {
                        length_str.push((*byte as char));
                    } else { break 'topl; }
                }
            }
        }
    } else {
        return Err(DecodeError { position: Some(position), kind: DecodeErrorKind::InvalidString } )
    }
    let length_int = try!(length_str.parse::<usize>());
    let string_data = &bytes[length_str.len()+1...(length_int+length_str.len())];
    let remaining = bytes[string_data.len()+length_str.len()+1..].to_vec();
    let bstring = BString::new(string_data);
    Ok((bstring, remaining))
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
                        return Err(DecodeError { position: None, kind: DecodeErrorKind::ExpectedByte('e') } )
                    }
                },
                _ => {
                    for i in &bytes[1..bytes.len()] {
                        if (*i as char) != 'e' {
                            number_string.push(*i as char);
                        } else { break; }
                    }
                },
            }
        } else {
            return Err(DecodeError { position: None, kind: DecodeErrorKind::ExpectedByte('i') } )
        }
    } else {
        return Err(DecodeError { position: None, kind: DecodeErrorKind::EndOfStream } )
    }
    let parsint = try!(number_string.parse::<u64>());
    let number = BInt::new(parsint);
    Ok(number)
}

pub fn blist_decode(bytes: Vec<u8>) -> Result<BList, DecodeError> {
    let mut result_list = BList::new();
    if bytes.len() > 1 {
        loop {
            if (bytes[0] as char) == 'l' {
                match bytes[1] as char {
                    'e' => {
                        return Ok(BList::new())
                    },
                    'i' => {
                        let bint = try!(bint_decode(bytes[1..bytes.len()].to_vec()));
                        result_list.push(Bencode::BInt(bint));
                        //bytes.remove 
                    },
                    'l' => {
                        let blist = try!(blist_decode(bytes[1..bytes.len()].to_vec()));
                        result_list.push(Bencode::BList(blist));
                    },
                    'd' => {
                        let bdict = try!(bdict_decode(bytes[1..bytes.len()].to_vec()));
                    },
                    _ => {
                        // Most likely a bencoded string
                        if (bytes[1] as char).is_digit(10) {
                            let bstring = try!(bstring_decode(bytes[1..bytes.len()].to_vec()));
                            result_list.push(Bencode::BString(bstring.0));
                        } else {
                            return Err(DecodeError { position: None, kind: DecodeErrorKind::UnknownType } )
                        }
                    },
                }
            } else {
                return Err(DecodeError { position: None, kind: DecodeErrorKind::ExpectedByte('l') } )
            }
            if (bytes[1] as char) == 'e' { break; }
        }
    } else {
        return Err(DecodeError { position: None, kind: DecodeErrorKind::EndOfStream } )
    }
    Ok(result_list)
}

pub fn bdict_decode(bytes: Vec<u8>) -> Result<BDict, DecodeError> {
    unimplemented!();
}

