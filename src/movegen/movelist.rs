use arrayvec::ArrayVec;

use crate::movegen::moves::Move;

// https://www.chessprogramming.org/Encoding_Moves#Move_Index
const MAX_MOVES: usize = 256;

pub struct MoveList(ArrayVec<Move, MAX_MOVES>);

impl Default for MoveList {
    fn default() -> Self {
        Self(ArrayVec::new())
    }
}

impl MoveList {
    pub fn push(&mut self, m: Move) {
        self.0.push(m);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, idx: usize) -> Move {
        self.0[idx]
    }
}
