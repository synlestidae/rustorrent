use bit_vec::BitVec;

mod local_file;
mod peer_file;

pub use file::local_file::{PartialFile, Piece};
pub use file::peer_file::*;

pub trait PartialFileTrait {
   fn length(&self) -> usize;
   fn has_piece(&self, i: usize) -> bool; 
   fn is_complete(&self) -> bool {
       let len = self.length();
       for i in 0..len {
           if !self.has_piece(i) {
                return false;
           }
       }
       true
   }
   fn bit_array(&self) -> BitVec {
       let len = self.length();
       let mut vec = BitVec::from_elem(len, false);
       for i in 0..len {
           if self.has_piece(i) {
               vec.set(i, true);
           }
       }
       vec
   }
}
