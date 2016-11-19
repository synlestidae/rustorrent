use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::io::{Read, Write};
use std::io;
use std::error::Error;
use std::fs::OpenOptions;
use std::path::Path;
use std::fs::File;

use mio::*;
use mio::tcp::TcpStream;
use mio::channel::channel;
use mio::channel::{Sender, Receiver};

use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;

use wire::handler::ServerHandler;
use wire::action::{PeerStreamAction};
use wire::msg::{PeerMsg, parse_peermsg, parse_handshake};
use wire::action::PeerId;
use wire::peer::PeerServer;
use wire::peer_info::PeerState;

const OUTSIDE_MSG: Token = Token(0);
pub type StreamId = u32;

pub struct Protocol {
    streams: HashMap<StreamId, (TcpStream, PeerState)>,
    handler: PeerServer,
    poll: Poll,
    sender: Sender<ChanMsg>,
    receiver: Receiver<ChanMsg>,
    info: MetaInfo,
    info_hash: SHA1Hash20b,
    next_peer_id: usize,
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16),
    ThrottleOn(usize),
    ThrottleOff,
}

impl Protocol {
    pub fn new(info: &MetaInfo,
               hash: SHA1Hash20b,
               our_peer_id: &str)
               -> (Protocol, Sender<ChanMsg>, Receiver<ChanMsg>) {
        let poll = Poll::new().unwrap();

        match (channel(), channel()) {
            ((to_inside, from_outside), (to_outside, from_inside)) => {

                poll.register(&from_outside,
                              OUTSIDE_MSG,
                              Ready::readable(),
                              PollOpt::level())
                    .unwrap();

                let proto = Protocol {
                    streams: HashMap::new(),
                    poll: poll,
                    sender: to_outside,
                    receiver: from_outside,
                    info: info.clone(),
                    info_hash: hash.clone(),
                    handler: ServerHandler::new(info.clone(), hash.clone(), our_peer_id),
                    next_peer_id: 1,
                };

                (proto, to_inside, from_inside)
            }
        }
    }

    pub fn run(&mut self) {
        const EVENT_CAPACITY: usize = 25;
        let mut events = Events::with_capacity(EVENT_CAPACITY);
        loop {
            self.poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                self._handle_event(event);
            }
        }
    }

    fn _handle_event(&mut self, event: Event) {
        match event.token() {
            OUTSIDE_MSG => {
                loop {
                    match self.receiver.try_recv() {
                        Ok(msg) => self._handle_outside_msg(msg),
                        _ => break,
                    }
                }
            }
            _ => self._handle_socket_event(event),
        }
    }

    fn _get_stream_id(&self, token: Token) -> Option<StreamId> {
        Some(match token {
            Token(id) => id as StreamId
        })
    }

    fn _handle_socket_event(&mut self, event: Event) {
        let kind = event.kind();

        let peer_id: StreamId = match self._get_stream_id(event.token()) {
            Some(p_id) => p_id,
            None => return,
        };

        let should_remove = if let Some(&mut (ref mut socket, ref mut peer)) = self.streams.get_mut(&peer_id) {
            info!("Got event {:?} from {:?}", event.kind(), socket.peer_addr());

            //read, and handle message if it parses
            if kind.is_readable() {
                peer.read_from_peer(socket);
                match peer.message() {
                    Some(msg) => {
                        self.handler.on_message_receive(peer, msg);
                    },
                    _ => ()
                }
            }

            //write any messages we have to peer
            if kind.is_writable() {
                peer.write_to_peer(socket);
            }

            //deregister so no more socket events
            if kind.is_hup() {
                peer.disconnect();
                self.poll.deregister(socket);
                true
            } else {
                false
            }
        } else {
            return;
        };

        //remove the peer from map if it disconnected
        if should_remove {
            self.streams.remove(&peer_id);
        }
    }

    fn _handle_outside_msg(&mut self, msg: ChanMsg) {
        match msg {
            ChanMsg::NewPeer(ip, port) => self._handle_new_peer(ip, port),
            _ => (),
        }
    }

    fn _handle_new_peer(&mut self, addr: IpAddr, port: u16) {
        for (id, &(ref socket, _)) in &self.streams {
            match (socket.peer_addr(), addr) {
                (Ok(SocketAddr::V4(peer)), IpAddr::V4(p)) => {
                    if peer.port() == port && peer.ip().octets() == p.octets() {
                        return;
                    }
                }
                (Ok(SocketAddr::V6(peer)), IpAddr::V6(p)) => unimplemented!(),
                (Err(e), _) => {
                    continue;
                } 
                _ => continue,
            }
        }

        match self._connect_to_peer(addr, port) {
            Some((sock, Token(id_usize))) => {
                let id = id_usize as u32;
                let num_pieces = self.info.info.pieces.len();

                
                let mut peer = PeerState::new(num_pieces, self.info.info.piece_length as usize, id);

                self.handler.on_peer_connect(&mut peer);
                self.streams.insert(id, (sock, peer));
            }
            None => (),
        }
    }

    fn _connect_to_peer(&mut self, addr: IpAddr, port: u16) -> Option<(TcpStream, Token)> {
        let sock_addr = SocketAddr::new(addr, port);
        let token = Token(self.next_peer_id);
        info!("Trying to connect to {} on port {}", addr, port);
        match TcpStream::connect(&sock_addr) {
            Ok(sock) => {
                self.poll
                    .register(&sock, token, Ready::all(), PollOpt::edge());

                self.next_peer_id += 1;
                return Some((sock, token));
            }
            Err(e) => {
                info!("Could not connect: {}", e);
                return None;
            }
        }
    }
}
