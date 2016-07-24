use std::collections::BTreeMap;
use std::{error, fmt};
use std::string::FromUtf8Error;
use std::str::Utf8Error;
use std::num::ParseIntError;

use self::DecodeErrorKind::*;

pub mod decode;
pub mod encode;

#[derive(Debug)]
pub struct BString(Vec<u8>);
pub struct BInt(u64);
pub struct BList(Vec<Bencode>);
pub struct BDict(BTreeMap<BString, Bencode>);

impl BString {
    pub fn new(bytes: &[u8]) -> BString {
        BString(bytes.to_vec())
    }

    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.0.clone())
    }
}

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
    Utf8Err(Utf8Error),
    IntParsingErr(ParseIntError),
}

impl fmt::Display for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().unwrap())
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(match self.kind {
            ExpectedByte(ref ch) => write!(f, "expected `{}`", ch),
            Utf8Err(ref u8e) => write!(f, "{}", u8e),
            IntParsingErr(ref intpe) => write!(f, "{}", intpe), 
        });
        match self.location {
            Some(ref l) => write!(f, " at location `{}`", l),
            None => Ok(())
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match self.kind {
            ExpectedByte(..) => "unexpected input byte",
            Utf8Err(..) => "failed with an utf8error",
            IntParsingErr(..) => "failed to parse integer",
        }
    }
}

impl From<FromUtf8Error> for DecodeError {
    fn from(utf8err: FromUtf8Error) -> DecodeError {
        DecodeError {
            location: None,
            kind: Utf8Err(utf8err.utf8_error()), 
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

