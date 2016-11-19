use std::time::SystemTime;
use file::PeerFile;
use wire::action::PeerId;
use std::io;
use std::io::{Read, Write};
use wire::msg::{PeerMsg, parse_peermsg};

pub struct PeerState {
    pub peer_id: PeerId,
    pub has_handshake: bool,
    pub disconnected: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
    pub am_choking: bool,
    pub am_interested: bool,
    pub last_msg_time: SystemTime,
    pub last_msg_sent_time: SystemTime,
    pub file: PeerFile,
    pub score: u64,
    pub connection_time: SystemTime,

    piece_size: usize,
    buffer: MessageBuffer
}

const MAX_BLOCK_SIZE: u64 = 2 << 14;
const MAX_BYTES_PER_REQUEST: u64 = 1024 * 512;
const MAX_PIECES_PEER: usize = 10;
const MAX_PIECE_SIZE: usize = 15 * 1024;

impl PeerState {
    pub fn new(len: usize, piece_size: usize, id: PeerId) -> PeerState {
        PeerState {
            peer_id: id,
            has_handshake: false,
            disconnected: false,
            peer_choking: true,
            peer_interested: false,
            am_choking: true,
            am_interested: false,
            last_msg_time: SystemTime::now(),
            last_msg_sent_time: SystemTime::now(),
            file: PeerFile::new(len),
            score: 0,
            connection_time: SystemTime::now(),
            buffer: MessageBuffer::new(),
            piece_size: piece_size
        }
    }

    pub fn interested(&mut self, flag: bool) {
        self.write_message_out(if flag { PeerMsg::Interested } else { PeerMsg::NotInterested });
        self.am_interested = flag;
    }

    pub fn choke(&mut self, flag: bool) {
        self.write_message_out(if flag { PeerMsg::Choke } else { PeerMsg::Unchoke });
        self.am_choking = flag;
    }

    pub fn send_piece_data(&mut self, index: u32, begin: u32, mut data: Vec<u8>) {
        if data.len() > MAX_PIECE_SIZE {
            let mut i = MAX_PIECE_SIZE;

            while data.len() > 0 {
                let mut rest_of_data = data.split_off(i);
                self.write_message_out(PeerMsg::Piece(index, (begin + i as u32), data));
                i += MAX_PIECE_SIZE;
                data = rest_of_data;
            }
        } else {
            self.write_message_out(PeerMsg::Piece(index, begin, data));
        }
    }

    pub fn request_piece(&mut self, piece_index: usize) {
        let piece_size = self.piece_size;
        if self.piece_size > MAX_PIECE_SIZE {
            let mut offset = MAX_PIECE_SIZE;
            while offset < self.piece_size {
                let length = if offset + MAX_PIECE_SIZE > piece_size { piece_size - offset } else { MAX_PIECE_SIZE };
                self.write_message_out(PeerMsg::Request(piece_index as u32, offset as u32, length as u32));
                offset += MAX_PIECE_SIZE;
            }
        } else {
            self.write_message_out(PeerMsg::Request(piece_index as u32, 0, piece_size as u32));
        }
    }

    pub fn send_have(&mut self, piece_index: usize) {
        self.write_message_out(PeerMsg::Have(piece_index as u32));
    }

    pub fn write_message_out(&mut self, msg: PeerMsg) {
        let bytes = msg.into();
        self.buffer._write_out(bytes)
    }

    pub fn disconnect(&mut self) {
        self.disconnected = true;
    } 

    pub fn message(&mut self) -> Option<PeerMsg> {
        let msg_result = self.buffer._message();

        if let Some(ref msg) = msg_result {
            match msg {
                &PeerMsg::Choke => self.peer_choking = true,
                &PeerMsg::Unchoke => self.peer_choking = false,
                &PeerMsg::Interested => self.peer_interested = true,
                &PeerMsg::NotInterested => self.peer_interested = false,
                &PeerMsg::Have(piece_index) => self.file.set(piece_index as usize, true),
                &PeerMsg::Bitfield(ref bitfield) => {
                    let limit = self.file.pieces.len();
                    for (i, bit) in bitfield.iter().enumerate() {
                        if i >= limit {
                            break;
                         }
                        self.file.set(i, bit);
                    }
                },
                _ => {}
            };
        }
        msg_result
    }

    pub fn write_to_peer(&mut self, sink: &mut Write) -> io::Result<usize> {
        self.buffer._take_out(sink)
    }

    pub fn read_from_peer(&mut self, sink: &mut Read) {
        self.buffer._read_in(sink);
    }
}


struct MessageBuffer {
    bytes_in: Vec<u8>,
    bytes_out: Vec<u8>,
}

impl MessageBuffer {
    fn new() -> MessageBuffer {
        MessageBuffer {
            bytes_in: Vec::new(),
            bytes_out: Vec::new()
        }
    }

    fn _write_in(&mut self, mut bytes: Vec<u8>) {
        self.bytes_in.append(&mut bytes);
    }

    fn _write_out(&mut self, mut bytes: Vec<u8>) {
        self.bytes_out.append(&mut bytes);
    }

    fn _message(&mut self) -> Option<PeerMsg> {
        if self.bytes_in.len() == 0 {
            return None;
        }

        match parse_peermsg(&self.bytes_in) {
            Ok((msg, offset)) => {
                if offset < self.bytes_in.len() {
                    self.bytes_in = self.bytes_in.split_off(offset);
                } else {
                    self.bytes_in = Vec::new();
                }
                Some(msg)
            }
            Err(err) => {
                // TODO It needs to distinguish between recoverable and non-recoverable
                None
            }
        }
    }

    fn _take_out(&mut self, out: &mut Write) -> io::Result<usize> {
        const MAX_BYTES_WRITE: usize = 1024 * 1024;
        let result = out.write(&self.bytes_out);
        match result {
            Ok(offset) => {
                self.bytes_out = self.bytes_out.split_off(offset);
            }
            _ => {}
        };
        result
    }

    fn _read_in(&mut self, src: &mut Read) -> io::Result<usize> {
        const MAX_BYTES_READ: usize = 1024 * 1024;

        //let result = src.read();
        let bytes_before = self.bytes_in.len();
        for byte in src.bytes() {
            match byte {
                Ok(b) => {
                    self.bytes_in.push(b);
                }
                Err(_) => break,
            }
        }

        Ok(self.bytes_in.len() - bytes_before)
    }
}