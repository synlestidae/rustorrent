use std::collections::BTreeMap;
use std::{error, fmt};
use std::string::FromUtf8Error;
use std::num::ParseIntError;
use self::DecodeErrorKind::*;

pub mod decode;
pub mod encode;

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct BString(Vec<u8>);
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct BInt(i64);
#[derive(Eq, PartialEq)]
pub struct BList(Vec<Bencode>);
#[derive(Eq, PartialEq, Debug)]
pub struct BDict(BTreeMap<BString, Bencode>);

// Makes it easier to access elements of BDict
impl BDict {
    pub fn get<'b>(&'b self, _key: &str) -> Option<&'b Bencode> {
        let s_bytes = _key.to_string().into_bytes();
        let key = BString::new(&s_bytes);
        self.0.get(&key)
    }
}

impl BString {
    pub fn new(bytes: &[u8]) -> BString {
        BString(bytes.to_vec())
    }

    pub fn from_str(s: &str) -> BString {
        BString(s.to_string().into_bytes())
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
}

#[derive(Eq, PartialEq)]
pub enum Bencode {
    BString(BString),
    BInt(BInt),
    BList(BList),
    BDict(BDict),
}

#[derive(Debug)]
pub struct DecodeError {
    position: Option<usize>,
    kind: DecodeErrorKind,
}

#[derive(Debug)]
pub enum DecodeErrorKind {
    ExpectedByte(char),
    EndOfStream,
    InvalidString,
    UnknownType,
    IntParsingErr(ParseIntError),
    IntNegativeZero,
    Utf8Err(FromUtf8Error)
}

impl fmt::Display for Bencode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bencode::BString(ref s) => write!(f, "{}", s),
            Bencode::BInt(..) => write!(f, "to be implemented"),
            Bencode::BList(ref l) => write!(f, "{:?}", l),
            Bencode::BDict(..) => write!(f, "to be implemented"),
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
        write!(f, "{}", self.to_string().unwrap())
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
