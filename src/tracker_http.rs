use tracker::{TrackerReq, TrackerResp};
use bencode::decode::belement_decode;
use hyper::Url;
use hyper::client::Request;
use hyper::client::Client;
use hyper::net::HttpStream;

pub trait TrackerHandler {
    fn request(req: &TrackerReq) -> TrackerResp;
}

pub struct HttpTrackerHandler {
    url: Url
}

impl HttpTrackerHandler {
    pub fn new(url: Url) -> HttpTrackerHandler {
        HttpTrackerHandler { url: url  } 
    }
}

impl TrackerHandler for HttpTrackerHandler {
    fn request(_: &TrackerReq) -> TrackerResp {
        let mut url = self.url.clone();
        unimplemented!()
    }
}
