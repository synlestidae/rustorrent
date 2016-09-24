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
}

impl Error for TrackerError {
    fn description(&self) -> &str {
        unimplemented!()
    }
}

impl fmt::Display for TrackerError {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

impl TrackerHandler for HttpTrackerHandler {
    fn request(&self, req: &TrackerReq) -> Result<TrackerResp, TrackerError> {
        // build the url
        let mut url = self.url.clone();
        let query_string = req.to_query_string_pairs()
            .iter()
            .fold(String::new(),
                  |string, &(ref k, ref v)| format!("{}{}={}", string, k, v));
        url.set_query(Some(&query_string));

        // make the request
        let client = Client::new();
        match client.get(url).send() {
            Ok(mut response) => {
                let mut response_bytes = Vec::new();
                response.read_to_end(&mut response_bytes);
                let response_dict = BDict::try_from(belement_decode(&response_bytes).unwrap().0)
                    .unwrap();
                let tracker_response: TrackerResp = TrackerResp::try_from(response_dict).unwrap();
                Ok(tracker_response)
            }
            Err(_) => Err(TrackerError::Unknown),
        }
    }
}
