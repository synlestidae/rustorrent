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
use wire::data::PeerMsg;
use wire::protocol_handler::PeerServer;
use wire::handler::PeerAction;
use wire::handler::ServerHandler;

const OUTSIDE_MSG: Token = Token(0);
type StreamId = u32;

pub struct Protocol {
    streams: HashMap<StreamId, (TcpStream, PeerStream, PeerServer)>,
    poll: Poll,
    sender: Sender<ChanMsg>,
    receiver: Receiver<ChanMsg>,
    info: MetaInfo,
    info_hash: SHA1Hash20b,
    pending_actions: Vec<PeerAction>
}

struct PeerStream {
    id: StreamId,
    bytes_in: Vec<u8>,
    bytes_out: Vec<u8>,
}

impl PeerStream {
    fn write_in(&mut self, bytes: Vec<u8>) {
        unimplemented!();
    }

    fn write_out(&mut self, bytes: Vec<u8>) {
        unimplemented!();
    }

    fn message(&mut self) -> Option<PeerMsg> {
        unimplemented!();
    }

    fn take(&mut self, out: &mut Write) -> bool {
        const MAX_BYTES_WRITE: usize = 1024 * 16; 
        let result = {
            let out_ref = if self.bytes_out.len() < MAX_BYTES_WRITE {
                &self.bytes_out
            } else {
                &self.bytes_out[0..MAX_BYTES_WRITE]
            };
            out.write(out_ref)
        };
        match result {
            Ok(0) => false,
            Ok(num_bytes) => {
                self.bytes_out.split_off(num_bytes - 1);
                true
            },
            _ => false
        }
    }
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16),
}

impl Protocol {
    pub fn new(info: &MetaInfo,
               hash: SHA1Hash20b)
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
                    info_hash: hash,
                    pending_actions: Vec::new()
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

        if let Some((mut tcp_stream, mut peer_stream, mut handler)) = self.streams.remove(&peer_id) {
            if kind.is_readable() {
                // read bytes of messages
                match Protocol::_handle_read(&mut tcp_stream, &mut peer_stream, &mut handler) {
                    Some(action) => unimplemented!(),
                    None => ()
                }
            }
            if kind.is_writable() {
                // write pending messages
                Protocol::_handle_write(&mut tcp_stream, &mut peer_stream, &mut handler)
            }
            if kind.is_hup() {
                // remove socket and clean up
                Protocol::_handle_hup(&mut tcp_stream, &mut peer_stream, &mut handler);
            }
            self.streams.insert(peer_stream.id, (tcp_stream, peer_stream, handler));
        }

    }

    fn _handle_read(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) ->
        Option<PeerAction> {

        let mut buf = Vec::new(); 
        match socket.read(&mut buf) {
            Ok(bytes) => {
                peer.write_in(buf);
            }
            _ => (),
        }

        peer.message().map(|msg| handler.on_message_receive(peer.id, msg))
    }

    fn _handle_write(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) {
        peer.take(socket);
    }

    fn _handle_hup(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) {}

    fn _handle_outside_msg(&mut self, msg: ChanMsg) {
        match msg {
            ChanMsg::NewPeer(ip, port) => self._connect_to_peer(ip, port),
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
