use bit_vec::BitVec;
use file::PartialFileTrait;

pub struct PeerFile {
    pieces: BitVec
}

impl PeerFile {
    pub fn new(length: usize) -> PeerFile {
        PeerFile {
            pieces: BitVec::from_elem(length, false)
        }
    }

    pub fn from(pieces: &BitVec) -> PeerFile {
        PeerFile {
            pieces: BitVec::from_elem(pieces.len(), false)
        }
    }

    pub fn set(&mut self, index: usize, flag: bool) {
        self.pieces.set(index, flag);
    }
}

impl PartialFileTrait for PeerFile {
    fn length(&self) -> usize {
        self.pieces.len()
    }

    fn has_piece(&self, i: usize) -> bool {
        self.pieces.get(i).unwrap_or(false)
    }
}
