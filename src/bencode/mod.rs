use std::collections::BTreeMap;
use std::{error, fmt};
use std::string::FromUtf8Error;
use std::num::ParseIntError;

use self::DecodeErrorKind::*;

pub mod decode;
pub mod encode;

#[derive(Debug)]
pub struct BString(Vec<u8>);
#[derive(Debug)]
pub struct BInt(u64);
#[derive(Debug)]
pub struct BList(Vec<Bencode>);
#[derive(Debug)]
pub struct BDict(BTreeMap<BString, Bencode>);

impl BString {
    pub fn new(bytes: &[u8]) -> BString {
        BString(bytes.to_vec())
    }

    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.0.clone())
    }
}

impl BInt {
    pub fn new(number: u64) -> BInt {
        BInt(number)
    }

    pub fn to_u64(&self) -> u64 {
        self.0
    }
}

impl BList {
    pub fn new() -> BList {
        let vector: Vec<Bencode> = Vec::new();
        BList(vector)
    }

    pub fn to_vec(self) -> Vec<Bencode> {
        self.0
    }

    pub fn push(&mut self, value: Bencode) {
        self.0.push(value);
    }
}

#[derive(Debug)]
pub enum Bencode {
    BString(BString),
    BInt(BInt),
    BList(BList),
    BDict(BDict),
}

#[derive(Debug)]
pub struct DecodeError {
    location: Option<String>,
    kind: DecodeErrorKind,
}

#[derive(Debug)]
pub enum DecodeErrorKind {
    ExpectedByte(char),
    EndOfStream, 
    InvalidString,
    UnknownType,
    IntParsingErr(ParseIntError),
}

impl fmt::Display for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().unwrap())
    }
}

impl fmt::Display for BInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_u64())
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
        });
        match self.location {
            Some(ref l) => write!(f, " at location `{}` of the byte stream", l),
            None => Ok(())
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
        }
    }
}

impl From<ParseIntError> for DecodeError {
    fn from(intperr: ParseIntError) -> DecodeError {
        DecodeError {
            location: None,
            kind: IntParsingErr(intperr), 
        }
    }
}

