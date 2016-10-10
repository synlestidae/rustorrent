use mio::*;
use mio::tcp::TcpStream;
use mio::channel::channel;
use mio::channel::{Sender, Receiver};
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::io::{Read, Write};
use std::error::Error;

use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;


use wire::handler::{ServerHandler, BasicHandler, PeerHandler, PeerAction, PeerStreamAction};
use wire::data::{PeerMsg, parse_peermsg};
use wire::protocol_handler::PeerServer;

const OUTSIDE_MSG: Token = Token(0);
type StreamId = u32;

pub struct Protocol {
    streams: HashMap<StreamId, (TcpStream, PeerStream)>,
    handler: PeerServer,
    poll: Poll,
    sender: Sender<ChanMsg>,
    receiver: Receiver<ChanMsg>,
    info: MetaInfo,
    info_hash: SHA1Hash20b,
    pending_actions: Vec<PeerAction>,
    stats: Stats,
    next_peer_id: usize,
}

struct PeerStream {
    id: StreamId,
    bytes_in: Vec<u8>,
    bytes_out: Vec<u8>,
    handshake_sent: bool,
    handshake_received: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct Stats {
    pub uploaded: usize,
    pub downloaded: usize,
    pub peer_count: usize,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            uploaded: 0,
            downloaded: 0,
            peer_count: 0,
        }
    }
}

impl PeerStream {
    fn new(id: StreamId) -> PeerStream {
        PeerStream {
            id: id,
            bytes_in: Vec::new(),
            bytes_out: Vec::new(),
            handshake_sent: false,
            handshake_received: false,
        }
    }

    fn write_in(&mut self, mut bytes: Vec<u8>) {
        self.bytes_in.append(&mut bytes);
    }

    fn write_out(&mut self, mut bytes: Vec<u8>) {
        self.bytes_out.append(&mut bytes);
    }

    fn message(&mut self) -> Option<PeerMsg> {
        if self.bytes_in.len() == 0 {
            return None;
        }

        match parse_peermsg(&self.bytes_in) {
            Ok((msg, offset)) => {
                if offset < self.bytes_in.len() {
                    self.bytes_in.split_off(offset);
                } else {
                    self.bytes_in = Vec::new();
                }
                Some(msg)
            }
            Err(_) => {
                // TODO It needs to distinguish between recoverable and non-recoverable
                None
            }
        }
    }

    fn take(&mut self, out: &mut Write) -> usize {
        const MAX_BYTES_WRITE: usize = 1024 * 128;

        let result = {
            let out_ref = if self.bytes_out.len() < MAX_BYTES_WRITE {
                &self.bytes_out
            } else {
                &self.bytes_out[0..MAX_BYTES_WRITE]
            };
            println!("Trying to write {} bytes", out_ref.len());
            out.write(out_ref)
        };

        match result {
            Ok(0) => 0,
            Ok(num_bytes) => {
                self.bytes_out.split_off(num_bytes - 1);
                num_bytes
            }
            Err(err) => {
                println!("Error writing to socket {:?}", err);
                0
            }
        }
    }
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16),
    StatsRequest,
    StatsResponse(Stats),
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
                    pending_actions: Vec::new(),
                    stats: Stats::new(),
                    handler: ServerHandler::new(info.clone(), hash.clone(), our_peer_id),
                    next_peer_id: 1,
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
        }
    }

    fn _get_stream_id(&self, token: Token) -> Option<StreamId> {
        Some(match token {
            Token(id) => id as StreamId,
        })
    }

    fn _perform_action(&mut self, action: PeerAction) {
        let id = action.0;
        match action.1 {
            PeerStreamAction::Nothing => (),
            PeerStreamAction::SendMessages(msgs) => {
                for msg in msgs {
                    let bytes: Vec<u8> = msg.into();
                    match self.streams.get_mut(&id) {
                        Some(&mut (_, ref mut peer)) => {
                            peer.write_out(bytes);
                        }
                        None => {
                            // this peer already disconnected
                        }
                    }
                }
            }
            PeerStreamAction::Disconnect => {
                self.streams.remove(&id);
            }
        }
    }

    fn _handle_socket_event(&mut self, event: Event) {
        println!("Event: {:?}", event);
        let kind = event.kind();
        let peer_id: StreamId = match self._get_stream_id(event.token()) {
            Some(p_id) => p_id,
            None => return,
        };

        if let Some((mut tcp_stream, mut peer_stream)) = self.streams.remove(&peer_id) {
            println!("Event for {:?}", tcp_stream.peer_addr());
            if kind.is_error() {
                let error = tcp_stream.take_error();
                println!("Some kind of error {:?}", error);
                if let Ok(err) = error {
                    if let Some(c) = err {
                        println!("Cause: {:?}", c.cause());
                    }
                }
            }

            if kind.is_writable() {
                println!("Oooooh, writable {:?}", event);
                // write pending messages
                let b_written =
                    Protocol::_handle_write(&mut tcp_stream, &mut peer_stream, &mut self.handler);
                println!("Wrote {} bytes", b_written);
            }

            if kind.is_readable() {
                println!("Oooooh, readable {:?}", event);
                // read bytes of messages
                let read_result =
                    Protocol::_handle_read(&mut tcp_stream, &mut peer_stream, &mut self.handler);
                match read_result.0 {
                    Some(action) => self._perform_action(action),
                    None => (),
                }
                self.stats.uploaded += read_result.1;
            }
            if kind.is_hup() || kind.is_error() {
                println!("Oooooh, disconnect {:?}", event);
                Protocol::_handle_hup(&mut tcp_stream, &mut peer_stream, &mut self.handler);
                self.poll.deregister(&tcp_stream);
                // to remove socket, only need to return early from this method
                return;
            }
            self.streams.insert(peer_stream.id, (tcp_stream, peer_stream));
        }

    }

    fn _handle_read(socket: &mut TcpStream,
                    peer: &mut PeerStream,
                    handler: &mut PeerServer)
                    -> (Option<PeerAction>, usize) {

        let mut buf = Vec::new();
        let bytes_read = match socket.read(&mut buf) {
            Ok(bytes_read) => {
                peer.write_in(buf);
                bytes_read
            }
            _ => 0,
        };

        let action = match peer.message() {
            Some(msg) => {
                println!("Received from peer {:?}", msg);
                Some(handler.on_message_receive(peer.id, msg))
            }
            _ => return (None, bytes_read),
        };

        (action, bytes_read)
    }

    fn _handle_write(socket: &mut TcpStream,
                     peer: &mut PeerStream,
                     handler: &mut PeerServer)
                     -> usize {
        peer.take(socket)
    }

    fn _handle_hup(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) {}

    fn _handle_outside_msg(&mut self, msg: ChanMsg) {
        match msg {
            ChanMsg::NewPeer(ip, port) => self._handle_new_peer(ip, port),
            ChanMsg::StatsRequest => {
                self.sender.send(ChanMsg::StatsResponse(self.stats));
            }
            _ => (),
        }
    }

    fn _handle_new_peer(&mut self, addr: IpAddr, port: u16) {
        let mut ids_to_remove = Vec::new();

        for (id, &(ref socket, _)) in &self.streams {
            match (socket.peer_addr(), addr) {
                (Ok(SocketAddr::V4(peer)), IpAddr::V4(p)) => {
                    if peer.port() == port && peer.ip().octets() == p.octets() {
                        return;
                    }
                }
                (Ok(SocketAddr::V6(peer)), IpAddr::V6(p)) => unimplemented!(),
                (Err(e), _) => {
                    // println!("Error: {}", e);
                    // self.poll.deregister(socket);
                    continue;
                } 
                _ => continue,
            }
        }

        for id in ids_to_remove {
            self.streams.remove(&id);
        }

        match self._connect_to_peer(addr, port) {
            Some(sock) => {
                let id = self.next_peer_id as u32;
                self.streams.insert(id, (sock, PeerStream::new(id)));
                let action = self.handler.on_peer_connect(id);
                self._perform_action(action);
                self.next_peer_id += 1;
            }
            None => (),
        }
    }

    fn _connect_to_peer(&mut self, addr: IpAddr, port: u16) -> Option<TcpStream> {
        let sock_addr = SocketAddr::new(addr, port);
        if let Ok(sock) = TcpStream::connect(&sock_addr) {
            self.poll
                .register(&sock,
                          Token(self.next_peer_id),
                          Ready::all(),
                          PollOpt::edge());
            println!("Connected to {} on port {}", addr, port);
            return Some(sock);
        }
        return None;
    }
}
