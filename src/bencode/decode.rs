use std::collections::BTreeMap;
use bencode::{Bencode, BString, BInt, BList, BDict, DecodeError, DecodeErrorKind};
use sha1::Sha1;

pub struct DecodeResult<T>(pub T, pub usize);

pub fn belement_decode(bytes: &[u8]) -> Result<DecodeResult<Bencode>, DecodeError> {
    if bytes.len() == 0 {
        return Err(DecodeError {
            position: Some(0),
            kind: DecodeErrorKind::EndOfStream,
        });
    }

    Ok(if bytes[0] == 'i' as u8 {
        let result = try!(bint_decode(bytes));
        DecodeResult(Bencode::BInt(result.0), result.1)
    } else if bytes[0] == 'l' as u8 {
        let result = try!(blist_decode(bytes));
        DecodeResult(Bencode::BList(result.0), result.1)
    } else if bytes[0] == 'd' as u8 {
        let result = try!(bdict_decode(bytes));
        DecodeResult(Bencode::BDict(result.0), result.1)
    } else {
        let result = try!(bstring_decode(bytes));
        DecodeResult(Bencode::BString(result.0), result.1)
    })
}

pub fn bstring_decode(bytes: &[u8]) -> Result<DecodeResult<BString>, DecodeError> {
    let mut position = 0;
    const ASCII_HEX_ZERO: u8 = 0x30;
    const ASCII_HEX_NINE: u8 = 0x39;

    if !(ASCII_HEX_ZERO <= bytes[position] && bytes[position] <= ASCII_HEX_NINE) {
        return Err(DecodeError {
            position: Some(position),
            kind: DecodeErrorKind::InvalidString,
        });
    }

    while ASCII_HEX_ZERO <= bytes[position] && bytes[position] <= 0x39 {
        position += 1;
    }


    if bytes[position] != ':' as u8 {
        return Err(DecodeError {
            position: Some(position),
            kind: DecodeErrorKind::InvalidString,
        });

    }

    let len_string = try!(String::from_utf8(bytes[0..position]
        .iter()
        .map(|&x| x)
        .collect::<Vec<u8>>()));
    let len = len_string.parse::<usize>().unwrap();
    position += 1;
    let str_bytes = bytes[position..(position + len)].iter().map(|&x| x).collect::<Vec<u8>>();
    position = position + len;
    Ok(DecodeResult(BString(str_bytes), position))
}

pub fn bint_decode(bytes: &[u8]) -> Result<DecodeResult<BInt>, DecodeError> {
    let mut position = 0;
    let mut number_string = String::new();
    if bytes.len() > 1 {
        if (bytes[position] as char) == 'i' {
            position += 1;
            match bytes[position] as char {
                '0' => {
                    if bytes.len() >= 2 && (bytes[position + 1] as char) == 'e' {
                        position = position + 2;
                        return Ok(DecodeResult(BInt::new(0i64), position));
                    } else {
                        return Err(DecodeError {
                            position: Some(position),
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
    position += number_string.len() + 1;
    Ok(DecodeResult(number, position))
}

pub fn blist_decode(bytes: &[u8]) -> Result<DecodeResult<BList>, DecodeError> {
    let mut position = 0;
    let mut list = Vec::new();
    if bytes.len() > 1 {
        if bytes[position] != 'l' as u8 {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::ExpectedByte('l'),
            });
        }
        position += 1;
        loop {
            if position >= bytes.len() {
                return Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::EndOfStream,
                });
            } else if bytes[position] == 'e' as u8 {
                position += 1;
                break;
            } else {
                let result = try!(belement_decode(&bytes[position..bytes.len()]));
                position += result.1;
                list.push(result.0);
            }
        }
        Ok(DecodeResult(BList(list), position))
    } else {
        return Err(DecodeError {
            position: None,
            kind: DecodeErrorKind::EndOfStream,
        });
    }
}

pub fn bdict_decode(bytes: &[u8]) -> Result<DecodeResult<BDict>, DecodeError> {
    let mut position = 0;
    if bytes[position] != 'd' as u8 {
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
    position += 1;
    loop {
        if position >= bytes.len() {
            return Err(DecodeError {
                position: None,
                kind: DecodeErrorKind::EndOfStream,
            });
        } else if bytes[position] == 'e' as u8 {
            position += 1;
            break;
        } else {
            let key = try!(bstring_decode(&bytes[position..bytes.len()]));
            position += key.1;
            let value_out = belement_decode(&bytes[position..bytes.len()]);
            let value = try!(value_out);
            position += value.1;
            map.insert(key.0, value.0);
        }
    }
    let mut sha1 = Sha1::new();
    sha1.update(&bytes[0..position]);
    let hash = sha1.digest().bytes().iter().map(|&x| x).collect::<Vec<u8>>();
    Ok(DecodeResult(BDict(map, hash), position))
}
