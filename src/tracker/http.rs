use tracker::data::{TrackerReq, TrackerResp};
use bencode::Bencode;
use bencode::decode::{belement_decode, DecodeResult};
use bencode::BDict;
use hyper::Url;
use hyper::client::Request;
use hyper::client::Client;
use hyper::net::HttpStream;
use std::error::Error;
use std::fmt;
use std::io::Read;
use convert::TryFrom;
use bencode::DecodeError;
use std::fs::File;
use std::io::Write;

pub trait TrackerHandler {
    fn request(self: &Self, req: &TrackerReq) -> Result<TrackerResp, TrackerError>;
}

pub struct HttpTrackerHandler {
    url: Url,
}

impl HttpTrackerHandler {
    pub fn new(url: Url) -> HttpTrackerHandler {
        HttpTrackerHandler { url: url }
    }
}

#[derive(Debug)]
pub enum TrackerError {
    Unknown,
    ParseError(DecodeError),
    Http(String),
}

impl Error for TrackerError {
    fn description(&self) -> &str {
        unimplemented!()
    }
}

impl fmt::Display for TrackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &TrackerError::Unknown => {
                f.write_str("An unknown error occurred");
            }
            &TrackerError::ParseError(ref e) => {
                f.write_str("Parse error: ");
                f.write_str(&format!("{}", e));
            }
            &TrackerError::Http(ref e) => {
                f.write_str(e);
            }
        };
        Ok(())
    }
}

impl TrackerHandler for HttpTrackerHandler {
    fn request(&self, req: &TrackerReq) -> Result<TrackerResp, TrackerError> {
        // build the url
        let mut url = self.url.clone();
        let query_string = req.to_query_string_pairs()
            .iter()
            .fold(String::new(),
                  |string, &(ref k, ref v)| format!("{}&{}={}", string, k, v));
        url.set_query(Some(&query_string));

        // make the request
        let client = Client::new();
        match client.get(url).send() {
            Ok(mut response) => {
                let mut response_bytes = Vec::new();
                response.read_to_end(&mut response_bytes);
                let response_dict =
                    match BDict::try_from(belement_decode(&response_bytes).unwrap().0) {
                        Ok(bdict) => bdict,
                        Err(error) => return Err(TrackerError::ParseError(error)),
                    };
                let tracker_response: TrackerResp = TrackerResp::try_from(response_dict).unwrap();
                Ok(tracker_response)
            }
            Err(e) => Err(TrackerError::Http(e.to_string())),
        }
    }
}
