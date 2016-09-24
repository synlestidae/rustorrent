use std::ops::{Index, IndexMut};
use sha1::{Sha1, Digest};
use metainfo::FileInfo;
use metainfo::SHA1Hash20b;

pub struct PartialFile {
    collection: PieceCollection,
    info: FileInfo
}

impl PartialFile {
    pub fn new(info: &FileInfo) -> PartialFile {
        PartialFile {
            info: info.clone(),
            collection: PieceCollection::new(&info.pieces,
                info.pieces.len() as u64)
        }
    }

    pub fn is_complete(&self) -> bool {
        for i in 0..self.info.pieces.len() {
            if !self._is_piece_complete(i) {
                return false;
            }
        }
        true
    }

    pub fn has_piece(&self, i: usize) -> bool {
        self._is_piece_complete(i)
    }

    pub fn get_piece<'a>(&'a self, i: usize) -> &'a Piece {
        &self.collection.pieces[i]
    }


    pub fn get_piece_mut<'a>(&'a mut self, i: usize) -> &'a mut Piece {
        &mut self.collection.pieces[i]
    }

    fn _is_piece_complete(&self, i: usize) -> bool {
        let mut sha1: Sha1 = Sha1::new();
        sha1.update(&self.collection[i].data); 
        let ref bytes1 = sha1.digest().bytes();
        let ref bytes2 = self.info.pieces[i];
        bytes1 == bytes2.as_slice()
    }


    pub fn bit_array(&self) -> Vec<bool> {
        (0..self.info.pieces.len())
            .map(|i| self._is_piece_complete(i))
            .collect::<Vec<bool>>()
    }

    pub fn add_piece(&mut self, index: usize, offset: usize, block: Vec<u8>) -> bool {
        self.collection.add(index, offset, block)
    }
}

pub struct Piece {
    data: Vec<u8>,
    length: u32,
    hash: SHA1Hash20b,
    definitely_complete: bool
}

impl Piece {
    pub fn new(length: u32, hash: SHA1Hash20b) -> Piece {
        Piece {
            data: Vec::new(),
            length: length,
            hash: hash,
            definitely_complete: false
        }
    }
    pub fn add(&mut self, offset: usize, block: &[u8]) -> bool {
        if offset as u32 > self.length || self.is_complete() {
            return false;
        }
        let existing_block = &mut self.data;
        existing_block.resize(offset + block.len(), 0);
        for i in 0..block.len() {
            existing_block[offset + i] = block[i];
        }
        true
    }

    pub fn get_offset<'a>(&'a mut self, begin: usize, offset: usize) -> Option<&'a [u8]> {
        let len = self.data.len();
        if begin + offset < len && self.is_complete() {
            Some(&self.data[begin..(begin+offset)])
        } else {
            None
        }
    }

    pub fn is_complete(&mut self) -> bool {
        if self.definitely_complete {
            return true;
        }
        let mut sha1: Sha1 = Sha1::new();
        sha1.update(&self.data); 
        let ref bytes1 = sha1.digest().bytes();
        let ref bytes2 = self.hash;
        let complete = bytes1 == bytes2.as_slice();
        self.definitely_complete = complete;
        complete
    }
}

struct PieceCollection {
    pieces: Vec<Piece>,
    piece_size: u64
}

impl PieceCollection {
    pub fn new(pieces: &[SHA1Hash20b], size: u64) -> PieceCollection {
        let mut vec = Vec::new();
        for hash in pieces {
            vec.push(Piece::new(size as u32, hash.clone()));
        }
        PieceCollection { pieces: vec, piece_size: size } 
    }

    pub fn add(&mut self, index: usize, offset: usize, block: Vec<u8>) -> bool {
        if index >= self.pieces.len() { return false; }
        if offset + block.len() > self.piece_size as usize {
            return false;
        }

        self.pieces[index].add(offset, &block);
        //existing_block.resize(offset + block.len(), 0);
        //for i in 0..block.len() {
        //    existing_block.data[offset + i as usize] = block[i];
        //}
        true
    }
}

impl Index<usize> for PieceCollection {
    type Output = Piece;

    fn index<'a>(&'a self, _index: usize) -> &'a Piece {
        &self.pieces[_index]
    }
}

impl IndexMut<usize> for PieceCollection {
    fn index_mut<'a>(&'a mut self, _index: usize) -> &'a mut Piece {
        &mut self.pieces[_index]
    }
}
