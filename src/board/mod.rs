use bitboard::Bitboard;

use crate::board::defs::{NUM_PIECES, NUM_SIDES};

mod bitboard;
mod defs;
mod square;

pub struct Board {
    pub bitboards: [[Bitboard; NUM_PIECES]; NUM_SIDES],
}
