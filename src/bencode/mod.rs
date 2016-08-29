use std::collections::BTreeMap;
use std::{error, fmt};
use std::string::FromUtf8Error;
use std::num::ParseIntError;
use self::DecodeErrorKind::*;
use std::convert::{From};
use convert::TryFrom;

pub mod decode;
pub mod encode;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct BString(Vec<u8>);
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct BInt(i64);
#[derive(Eq, PartialEq, Clone)]
pub struct BList(Vec<Bencode>);
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct BDict(BTreeMap<BString, Bencode>);

impl TryFrom<Bencode> for BInt {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        match element {
            Bencode::BInt(bint) => Ok(bint),
            _ => {
                Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
            }
        }
    }
}

/*impl TryFrom<Bencode> for String {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        match element {
            Bencode::BString(belement) => belement.to_string().map_err(|_| DecodeError {
                position: None, 
                kind: DecodeErrorKind::ConversionError
            }),
            _ => {
                Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
            }
        }
    }

}*/

impl TryFrom<Bencode> for BString {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        match element {
            Bencode::BString(belement) => Ok(belement),
            _ => {
                Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
            }
        }
    }
}

impl TryFrom<Bencode> for String {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        let error = DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                };

        match element {
            Bencode::BString(belement) => belement.to_string().map_err(|_| error),
            _ => Err(error)
        }
    }
}
impl TryFrom<Bencode> for BDict {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        match element {
            Bencode::BDict(belement) => Ok(belement),
            _ => {
                Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
            }
        }
    }
}

impl TryFrom<Bencode> for BList {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Self, Self::Err> {
        match element {
            Bencode::BList(belement) => Ok(belement),
            _ => {
                Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
            }
        }
    }
}

impl<A: TryFrom<Bencode>> TryFrom<Bencode> for Vec<A> {
    type Err = DecodeError;
    fn try_from(element: Bencode) -> Result<Vec<A>, Self::Err> {
        let mut result = Vec::new();
        match element {
            Bencode::BList(blist) => {
                for item in blist.0.into_iter() {
                    match A::try_from(item) {
                        Ok(item_a) => result.push(item_a),
                        _ => return Err(DecodeError {
                            position: None,
                            kind: DecodeErrorKind::ConversionError,
                        })
                    }
                }
                Ok(result)
            },
            _ => Err(DecodeError {
                    position: None,
                    kind: DecodeErrorKind::ConversionError,
                })
        }
    }
}


// Makes it easier to access elements of BDict
impl BDict {
    pub fn get<'b>(&'b self, _key: &str) -> Option<&'b Bencode> {
        let s_bytes = _key.to_string().into_bytes();
        let _key = BString::new(&s_bytes);
        self.0.get(&_key)
    }

    pub fn get_copy<A: TryFrom<Bencode>>(&self, key: &str) -> Option<A> {
        match self.get(key) {
            Some(&ref element) => A::try_from(element.clone()).ok(),
            _ => None
        }
    }
}

impl BString {
    pub fn new(bytes: &[u8]) -> BString {
        BString(bytes.to_vec())
    }

    pub fn from_str(s: &str) -> BString {
        BString(s.to_string().into_bytes())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.0.clone())
    }
}

impl BInt {
    pub fn new(number: i64) -> BInt {
        BInt(number)
    }

    pub fn to_i64(&self) -> i64 {
        self.0
    }
}

impl BList {
    pub fn new() -> BList {
        let vector: Vec<Bencode> = Vec::new();
        BList(vector)
    }
    pub fn from(src_list: Vec<Bencode>) -> BList {
        BList(src_list)
    }

    pub fn push(&mut self, value: Bencode) {
        self.0.push(value);
    }

    pub fn list<'a>(&'a self) -> &'a Vec<Bencode> {
        &self.0
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum Bencode {
    BString(BString),
    BInt(BInt),
    BList(BList),
    BDict(BDict),
}

#[derive(Debug)]
pub struct DecodeError {
    pub position: Option<usize>,
    pub kind: DecodeErrorKind,
}

#[derive(Debug)]
pub enum DecodeErrorKind {
    ExpectedByte(char),
    EndOfStream,
    InvalidString,
    UnknownType,
    IntParsingErr(ParseIntError),
    IntNegativeZero,
    Utf8Err(FromUtf8Error),
    ConversionError,
    MissingField(String)
}

impl fmt::Display for Bencode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bencode::BString(ref s) => write!(f, "{}", s),
            Bencode::BInt(BInt(bint)) => write!(f, "{}", bint),
            Bencode::BList(ref l) => write!(f, "{:?}", l),
            Bencode::BDict(BDict(ref bdict_map)) => write!(f, "{:?}", bdict_map),
        }
    }
}

impl fmt::Debug for Bencode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}",
               match self.to_string() {
                   Ok(string) => string,
                   Err(_) => format!("{:?}", self.0),
               })
    }
}

impl fmt::Debug for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for BInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_i64())
    }
}

impl fmt::Debug for BInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Debug for BList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(match self.kind {
            ExpectedByte(ref ch) => write!(f, "expected `{}`", ch),
            EndOfStream => write!(f, "reached end of input"),
            InvalidString => write!(f, "not a valid string"),
            UnknownType => write!(f, "type not recognised as bencoded"),
            IntParsingErr(ref intpe) => write!(f, "{}", intpe), 
            IntNegativeZero => write!(f, "-0 is not a valid integer"),
            Utf8Err(ref u8e) => write!(f, "{}", u8e),
            ConversionError => write!(f, "cannot convert type"),
            MissingField(ref field) => write!(f, "required field '{}' is missing on dictionary", field)
        });
        match self.position {
            Some(ref l) => write!(f, " at byte `{}` of the input stream", l),
            None => Ok(()),
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match self.kind {
            ExpectedByte(..) => "unexpected input byte",
            EndOfStream => "end of input, no more bytes",
            InvalidString => "failed to parse bytes as a string",
            UnknownType => "cannot parse as a valid bencoded type",
            IntParsingErr(..) => "failed to parse integer",
            IntNegativeZero => "-0 is not a valid integer",
            Utf8Err(..) => "failed with an utf8error",
            ConversionError => "failed to convert type",
            MissingField(..) => "required field is missing"
        }
    }
}

impl From<ParseIntError> for DecodeError {
    fn from(intperr: ParseIntError) -> DecodeError {
        DecodeError {
            position: None,
            kind: IntParsingErr(intperr),
        }
    }
}

impl From<FromUtf8Error> for DecodeError {
    fn from(utf8err: FromUtf8Error) -> DecodeError {
        DecodeError {
            position: None,
            kind: Utf8Err(utf8err),
        }
    }
}
