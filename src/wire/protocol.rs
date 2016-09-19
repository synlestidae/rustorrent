use mio::*;
use mio::tcp::{TcpStream};
use mio::channel::channel;
use mio::channel::{Sender, Receiver};
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;
use wire::handler::{BasicHandler, PeerHandler};

const OUTSIDE_MSG: Token = Token(0);

pub struct Protocol<H: PeerHandler> {
    streams: HashMap<usize, TcpStream>,
    poll: Poll,
    sender: Sender<ChanMsg>,
    receiver: Receiver<ChanMsg>,
    info: MetaInfo,
    info_hash: SHA1Hash20b,
    handlers: Vec<H>
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16)
}

impl<H: PeerHandler> Protocol<H> {
    pub fn new(info: &MetaInfo, hash: SHA1Hash20b) -> (Protocol<H>, Sender<ChanMsg>, Receiver<ChanMsg>) {
        let poll = Poll::new().unwrap();
        
        match (channel(), channel()) {
            ((to_inside, from_outside), (to_outside, from_inside)) => {

                poll.register(&from_outside, OUTSIDE_MSG, Ready::readable(),
                    PollOpt::edge()).unwrap();

                let proto = Protocol {
                    streams: HashMap::new(),
                    poll: poll, 
                    sender: to_outside,
                    receiver: from_outside,
                    info: info.clone(),
                    info_hash: hash,
                    handlers: Vec::new()
                };

                (proto, to_inside, from_inside)
            }
        }
    }

    pub fn run(&mut self) {
        let mut events = Events::with_capacity(1024);
        loop {
             self.poll.poll(&mut events, None).unwrap();
             for event in events.iter() {
                 match event.token() {
                     OUTSIDE_MSG => {
                         match self.receiver.try_recv() {
                             Ok(msg) => self._handle_outside_msg(msg),
                             _ => ()
                         }
                     },
                     _ => println!("Oooooh better do something!")
                 }
             }
        }
    }

    fn _handle_outside_msg(&mut self, msg: ChanMsg) {
        match msg {
            ChanMsg::NewPeer(ip, port) => self._connect_to_peer(ip, port)
        }
    }

    fn _connect_to_peer(&mut self, addr: IpAddr, port: u16) {
        let sock_addr = SocketAddr::new(addr, port);
        let sock = TcpStream::connect(&sock_addr).unwrap();
        self.poll.register(&sock, Token(1), Ready::readable() | Ready::writable(), PollOpt::edge()).unwrap();
    }
}
