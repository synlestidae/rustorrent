use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::io::{Read, Write};
use std::io;
use std::error::Error;

use mio::*;
use mio::tcp::TcpStream;
use mio::channel::channel;
use mio::channel::{Sender, Receiver};

use metainfo::MetaInfo;
use metainfo::SHA1Hash20b;

use wire::handler::{ServerHandler};
use wire::action::{PeerAction, PeerStreamAction};
use wire::msg::{PeerMsg, parse_peermsg, parse_handshake};
use wire::action::PeerId;
use wire::peer::PeerServer;

const OUTSIDE_MSG: Token = Token(0);
pub type StreamId = u32;

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

pub struct PeerStream {
    id: StreamId,
    bytes_in: Vec<u8>,
    bytes_out: Vec<u8>,
    handshake_sent: bool,
    handshake_received: bool,
    disconnected: bool,
}

impl PeerStream {
    pub fn new(id: StreamId) -> PeerStream {
        PeerStream {
            id: id,
            bytes_in: Vec::new(),
            bytes_out: Vec::new(),
            handshake_sent: true,
            handshake_received: false,
            disconnected: false,
        }
    }

    pub fn write_in(&mut self, mut bytes: Vec<u8>) {
        self.bytes_in.append(&mut bytes);
    }

    pub fn write_out(&mut self, mut bytes: Vec<u8>) {
        info!("This goes out {:?}", bytes);
        self.bytes_out.append(&mut bytes);
    }

    pub fn message(&mut self) -> Option<PeerMsg> {
        info!("Bytes in: {}", self.bytes_in.len());
        if self.bytes_in.len() == 0 {
            return None;
        }

        match parse_peermsg(&self.bytes_in) {
            Ok((msg, offset)) => {
                println!("Parsed a message of len {}", offset);
                if offset < self.bytes_in.len() {
                    self.bytes_in = self.bytes_in.split_off(offset);
                } else {
                    self.bytes_in = Vec::new();
                }
                info!("Message: {:?}", msg);
                Some(msg)
            }
            Err(err) => {
                info!("Message not ready {:?}", err);
                // TODO It needs to distinguish between recoverable and non-recoverable
                None
            }
        }
    }

    pub fn take(&mut self, out: &mut Write) -> io::Result<usize> {
        const MAX_BYTES_WRITE: usize = 1024 * 128;
        info!("Attempting to write {} bytes", self.bytes_out.len());
        let result = out.write(&self.bytes_out);
        match result {
            Ok(offset) => {
                info!("Wrote {} bytes", offset);
                self.bytes_out = self.bytes_out.split_off(offset);
            }
            Err(ref err) => {
                info!("Writing failed: {}", err);
            }
        };
        result
    }

    pub fn len_in(&self) -> usize {
        self.bytes_in.len()
    }
}

#[derive(Debug)]
pub enum ChanMsg {
    NewPeer(IpAddr, u16),
    StatsRequest,
    StatsResponse(Stats),
    ThrottleOn(usize),
    ThrottleOff
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
        const EVENT_CAPACITY: usize = 32;
        let mut events = Events::with_capacity(EVENT_CAPACITY);
        loop {
            self.poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                self._handle_event(event);
            }
            info!("All events handled");
            let actions = self.handler.on_loop();
            for action in actions {
                match self.streams.get_mut(&action.0) {
                    Some(&mut (_, ref mut peer)) => {
                        Protocol::_perform_action(action, peer);
                    }
                    None => ()
                }
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
            Token(id) => id as StreamId,
        })
    }

    fn _perform_action(action: PeerAction, peer: &mut PeerStream) {
        let id = action.0;
        match action.1 {
            PeerStreamAction::Nothing => (),
            PeerStreamAction::SendMessages(msgs) => {
                for msg in msgs {
                    info!("Writing message {:?}", msg);
                    let bytes = msg.into();
                    peer.write_out(bytes);
                }
            }
            PeerStreamAction::Disconnect => {
                peer.disconnected = true;
            }
        }
    }

    fn _handle_socket_event(&mut self, event: Event) {
        let kind = event.kind();
        let peer_id: StreamId = match self._get_stream_id(event.token()) {
            Some(p_id) => p_id,
            None => return,
        };

        if let Some((mut tcp_stream, mut peer_stream)) = self.streams.remove(&peer_id) {
            info!("Got event {:?} from peer id {}", event, peer_id);
            if (kind.is_readable()) {
                let read_result =
                    Protocol::_handle_read(&mut tcp_stream, &mut peer_stream, &mut self.handler);

                for action in read_result.0 {
                    Protocol::_perform_action(action, &mut peer_stream);
                }

                self.stats.uploaded += read_result.1;
            }

            let mut fatal_error_happened = false;
            if kind.is_writable() {
                // write pending messages
                let write_result =
                    Protocol::_handle_write(&mut tcp_stream, &mut peer_stream, &mut self.handler);

                if write_result.is_err() {
                    fatal_error_happened = true;
                }
            }

            if kind.is_hup() || kind.is_error() || fatal_error_happened {
                info!("Disconnected from peer id {}", peer_stream.id);
                self.poll.deregister(&tcp_stream);
                Protocol::_handle_hup(&mut tcp_stream, &mut peer_stream, &mut self.handler);

                // do not return socket to collection
                return;
            }

            self.streams.insert(peer_stream.id, (tcp_stream, peer_stream));
        }

    }

    fn _handle_read(socket: &mut TcpStream,
                    peer: &mut PeerStream,
                    handler: &mut PeerServer)
                    -> (Vec<PeerAction>, usize) {


        const READ_BUF_SIZE: usize = 1024;
        let mut buf = Vec::with_capacity(READ_BUF_SIZE);
        buf.resize(READ_BUF_SIZE, 0);
        let bytes_read = match socket.read(&mut buf) {
            Ok(bytes_read) => {
                peer.write_in(buf);
                info!("Read {} bytes from {}", bytes_read, peer.id);//, socket.peer_addr());
                bytes_read
            }
            Err(err) => 0,
        };


        // Attempt to parse a handshake first


        let mut actions = Vec::new();
        loop {
            let msg = if !peer.handshake_received {
                match parse_handshake(&peer.bytes_in) {
                    Ok((handshake, offset)) => {
                        peer.bytes_in = peer.bytes_in.split_off(offset);
                        peer.handshake_received = true;
                        info!("Handshake received: {:?}", handshake);
                        Some(handshake)
                    }
                    Err(e) => {
                        info!("Error parsing handshake: {:?}", e);
                        None
                    }
                }
            } else {
                peer.message()
            };

            let actions = match msg {
                Some(msg) => {
                    info!("Received from peer {:?}", msg);
                    actions.push(handler.on_message_receive(peer.id, msg));
                }
                None => {
                    info!("Error parsing. {} bytes left", peer.bytes_in.len());
                    return (Vec::new(), bytes_read);
                }
            };
        }

        info!("Read {} bytes", bytes_read);
        (actions, bytes_read)
    }

    fn _handle_write(socket: &mut TcpStream,
                     peer: &mut PeerStream,
                     handler: &mut PeerServer)
                     -> io::Result<usize> {
        peer.take(socket)
    }

    fn _handle_hup(socket: &mut TcpStream, peer: &mut PeerStream, handler: &mut PeerServer) {
        handler.on_peer_disconnect(peer.id);
    }

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
                let mut stream = PeerStream::new(id);
                let action = self.handler.on_peer_connect(id);

                Protocol::_perform_action(action, &mut stream);

                self.streams.insert(id, (sock, stream));
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
