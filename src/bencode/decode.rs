use std::collections::BTreeMap;
use bencode::{Bencode, BString, BInt, BList, BDict, DecodeError, DecodeErrorKind};

pub fn belement_decode(bytes: &[u8], position: &mut usize) -> Result<Bencode, DecodeError> {
    if bytes.len() == 0 || *position >= bytes.len() {
        return Err(DecodeError {
            position: Some(*position),
            kind: DecodeErrorKind::EndOfStream,
        });
    }

    if bytes[*position] == 'i' as u8 {
        let result = try!(bint_decode(bytes, position));
        Ok(Bencode::BInt(result))
    } else if bytes[*position] == 'l' as u8 {
        let result = try!(blist_decode(bytes, position));
        Ok(Bencode::BList(result))
    } else if bytes[*position] == 'd' as u8 {
        let result = try!(bdict_decode(bytes, position));
        Ok(Bencode::BDict(result))
    } else {
        let result = try!(bstring_decode(bytes, position));
        Ok(Bencode::BString(result))
    }
}

pub fn bstring_decode(bytes: &[u8], position_arg: &mut usize) -> Result<BString, DecodeError> {
    let mut position = *position_arg;
    if !(bytes[position] as char).is_numeric() {
        return Err(DecodeError {
            position: Some(position ),
            kind: DecodeErrorKind::InvalidString,
        });
    }
    while (bytes[position] as char).is_numeric() {
        position += 1;
    }
    let len = &String::from_utf8(bytes[*position_arg..position]
            .iter()
            .map(|&x| x)
            .collect::<Vec<u8>>())
        .unwrap()
        .parse::<usize>()
        .unwrap();
    if bytes[position] != ':' as u8 {
        return Err(DecodeError {
            position: Some(position ),
            kind: DecodeErrorKind::InvalidString,
        });

    }
    position += 1;
    let str_bytes = bytes[position..(position + len)].iter().map(|&x| x).collect::<Vec<u8>>();
    *position_arg = position + len;
    Ok(BString(str_bytes))
}

pub fn bint_decode(bytes: &[u8], position_arg: &mut usize) -> Result<BInt, DecodeError> {
    let mut position = *position_arg;
    let mut number_string = String::new();
    if bytes.len() > 1 {
        if (bytes[position] as char) == 'i' {
            position += 1;
            match bytes[position] as char {
                '0' => {
                    if bytes.len() >= 2 && (bytes[position + 1] as char) == 'e' {
                        *position_arg = *position_arg + 3;
                        return Ok(BInt::new(0i64));
                    } else {
                        return Err(DecodeError {
                            position: Some(*position_arg ),
                            kind: DecodeErrorKind::ExpectedByte('e'),
                        });
                    }
                }
                _ => {
                    for i in &bytes[position..bytes.len()] {
                        if (*i as char) != 'e' {
                            number_string.push(*i as char);
                        } else {
                            break;
                        }
                    }
                }
            }
        } else {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::ExpectedByte('i'),
            });
        }
    } else {
        return Err(DecodeError {
            position: None,
            kind: DecodeErrorKind::EndOfStream,
        });
    }

    if &number_string == "-0" {
        return Err(DecodeError {
            position: None,
            kind: DecodeErrorKind::IntNegativeZero,
        });
    }

    let parsint = try!(number_string.parse::<i64>());
    let number = BInt::new(parsint);
    *position_arg += number_string.len() + 2;
    Ok(number)
}

pub fn blist_decode(bytes: &[u8], position: &mut usize) -> Result<BList, DecodeError> {
    let mut list = Vec::new();
    if bytes.len() > 1 {
        if bytes[*position] != 'l' as u8 {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::ExpectedByte('l'),
            });
        }
        *position += 1;
        while *position < bytes.len() && bytes[*position] != 'e' as u8 {
            let result = try!(belement_decode(bytes, position));
            list.push(result);
        }
        if *position >= bytes.len() {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::EndOfStream,
            });

        }
        if bytes[*position] != 'e' as u8 {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::ExpectedByte('e'),
            });
        }
        Ok(BList(list))
    } else {
        return Err(DecodeError {
            position: None,
            kind: DecodeErrorKind::EndOfStream,
        });
    }
}

pub fn bdict_decode(bytes: &[u8], position: &mut usize) -> Result<BDict, DecodeError> {
    if bytes[*position] != 'd' as u8 {
        return Err(DecodeError {
            position: Some(0),
            kind: DecodeErrorKind::ExpectedByte('d'),
        });
    } else if bytes.len() < 2 {
        return Err(DecodeError {
            position: Some(1),
            kind: DecodeErrorKind::EndOfStream,
        });
    }
    let mut map = BTreeMap::new();
    *position += 1;
    while *position < bytes.len() && bytes[*position] != 'e' as u8 {
        let s_position = *position;
        let key = try!(bstring_decode(bytes, position));
        let value = try!(belement_decode(bytes, position));
        match key.to_string() {
            Ok(_) => map.insert(key, value),
            Err(e) => {
                return Err(DecodeError {
                    position: Some(s_position ),
                    kind: DecodeErrorKind::InvalidString,
                });
            }
        };
    }
    Ok(BDict(map))
}
