use std::io::{Read};
use wire::data::PeerMsg;
use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};
use byteorder::{BigEndian, ReadBytesExt};
use byteorder::ByteOrder;


struct StreamContainer<S: Read + Send> {
   src: S 
}

impl<S: Read + Send> StreamContainer<S> {
    pub fn new(src: S) -> StreamContainer<S> {
        StreamContainer {
            src: src
        }
    }

    fn _read_message_bytes(&mut self) -> Vec<u8> {
        let mut buf = vec![0, 0, 0, 0];
        self.src.read_exact(&mut buf);
        let size = BigEndian::read_u32(&buf);
        let mut message_vec: Vec<u8> = Vec::with_capacity(size as usize);
        for i in 0..size {
            message_vec.push(0);
        }
        self.src.read_exact(&mut message_vec);
        buf.append(&mut message_vec);
        buf
    }

    pub fn run(mut self, tx: Sender<Vec<u8>>) {
        loop {
            let bytes = self._read_message_bytes();
            tx.send(bytes);
        }
    }

}

pub struct PeerManager {
    src: Receiver<Vec<u8>>
}


pub type Err = ();
impl PeerManager {
    pub fn from<S: Read>(src: S) -> PeerManager where S: Send {
        let (tx, rx) = channel();
        let container = StreamContainer::new(src);
        let job = move || container.run(tx);
        thread::spawn(job);
        PeerManager { src: rx }
    }

    pub fn send_message(&self, msg: PeerMsg) -> Result<PeerMsg, Err> {
        unimplemented!();
    }
    
    pub fn message_available(&self) -> bool {
        unimplemented!();
    }

    pub fn wait_message(&self) -> Result<PeerMsg, Err> {
        unimplemented!();
    }

}
