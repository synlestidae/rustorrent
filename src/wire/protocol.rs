use mio::*;
use mio::tcp::{TcpStream};
use mio::channel::channel;
use mio::channel::{Sender, Receiver};
use std::collections::HashMap;
use std::net::IpAddr;
use metainfo::MetaInfo;

const OUTSIDE_MSG: Token = Token(0);

pub struct Protocol {
    streams: HashMap<usize, TcpStream>,
    poll: Poll,
    sender: Sender<ChanMsg>,
    receiver: Receiver<ChanMsg>
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr)
}

impl Protocol {
    pub fn new(info: &MetaInfo) -> (Protocol, Sender<ChanMsg>, Receiver<ChanMsg>) {
        let poll = Poll::new().unwrap();

        
        match (channel(), channel()) {
            ((to_inside, from_outside), (to_outside, from_inside)) => {

                poll.register(&from_outside, OUTSIDE_MSG, Ready::readable(),
                    PollOpt::edge()).unwrap();

                let proto = Protocol {
                    streams: HashMap::new(),
                    poll: poll, 
                    sender: to_outside,
                    receiver: from_outside
                };

                (proto, to_inside, from_inside)
            }
        }
    }

    pub fn run(&mut self) {
    }
}
