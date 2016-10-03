use mio::*;
use mio::tcp::TcpStream;
use mio::channel::channel;
use mio::channel::{Sender, Receiver};
use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::io::{Read, Write};

use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;


use wire::handler::{BasicHandler, PeerHandler};
use wire::data::{PeerMsg, parse_peermsg};
use wire::protocol_handler::PeerServer;
use wire::handler::PeerAction;
use wire::handler::ServerHandler;

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
    stats: Stats
}

struct PeerStream {
    id: StreamId,
    bytes_in: Vec<u8>,
    bytes_out: Vec<u8>,
    handshake_sent: bool,
    handshake_received: bool
}

#[derive(Debug, Copy, Clone)]
pub struct Stats {
    pub uploaded: usize,
    pub downloaded: usize,
    pub peer_count: usize
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            uploaded: 0,
            downloaded: 0,
            peer_count: 0
        }
    }
}

impl PeerStream {
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
            },
            Err(_) => {
                //TODO It needs to distinguish between recoverable and non-recoverable
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
            out.write(out_ref)
        };

        match result {
            Ok(0) => 0,
            Ok(num_bytes) => {
                self.bytes_out.split_off(num_bytes - 1);
                num_bytes
            },
            _ => 0
        }
    }
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16),
    StatsRequest,
    StatsResponse(Stats)
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
                              PollOpt::edge())
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
                    handler: ServerHandler::new(info.clone(), hash.clone(), our_peer_id)
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
                            _ => (),
                        }
                    }
                    _ => self._handle_socket_event(event),
                }
            }
        }
    }

    fn _get_stream_id(&self, token: Token) -> Option<StreamId> {
        unimplemented!()
    }

    fn _handle_socket_event(&mut self, event: Event) {
        let kind = event.kind();
        let peer_id: StreamId = match self._get_stream_id(event.token()) {
            Some(p_id) => p_id,
            None => return,
        };

        if let Some((mut tcp_stream, mut peer_stream)) = self.streams.remove(&peer_id) {
            if kind.is_readable() {
                // read bytes of messages
                let read_result = Protocol::_handle_read(&mut tcp_stream, &mut peer_stream, &mut self.handler);
                match read_result.0 {
                    Some(action) => unimplemented!(),
                    None => ()
                }
                self.stats.uploaded += read_result.1;
            }
            if kind.is_writable() {
                // write pending messages
                Protocol::_handle_write(&mut tcp_stream, &mut peer_stream, &mut self.handler);
            }
            if kind.is_hup() {
                Protocol::_handle_hup(&mut tcp_stream, &mut peer_stream, &mut self.handler);
                // to remove socket, only need to return early from this method 
                return;
            }
            self.streams.insert(peer_stream.id, (tcp_stream, peer_stream));
        }

    }

    fn _handle_read(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) ->
        (Option<PeerAction>, usize) {

        let mut buf = Vec::new(); 
        let bytes_read = match socket.read(&mut buf) {
            Ok(bytes_read) => {
                peer.write_in(buf);
                bytes_read
            }
            _ => 0
        };

        let action = peer.message().map(|msg| handler.on_message_receive(peer.id, msg));
        (action, bytes_read)
    }

    fn _handle_write(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) ->
        usize {
        peer.take(socket)
    }

    fn _handle_hup(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) {}

    fn _handle_outside_msg(&mut self, msg: ChanMsg) {
        match msg {
            ChanMsg::NewPeer(ip, port) => self._connect_to_peer(ip, port),
            ChanMsg::StatsRequest => { self.sender.send(ChanMsg::StatsResponse(self.stats)); }
            _ => ()
        }
    }

    fn _connect_to_peer(&mut self, addr: IpAddr, port: u16) {
        let sock_addr = SocketAddr::new(addr, port);
        let sock = TcpStream::connect(&sock_addr).unwrap();
        self.poll
            .register(&sock,
                      Token(1),
                      Ready::readable() | Ready::writable(),
                      PollOpt::edge())
            .unwrap();
    }
}
