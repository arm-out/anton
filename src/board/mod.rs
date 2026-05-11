use bitboard::Bitboard;

use crate::board::defs::{NUM_PIECES, NUM_SIDES};

pub mod bitboard;
mod defs;
pub mod square;

pub struct Board {
    pub bitboards: [[Bitboard; NUM_PIECES]; NUM_SIDES],
}
