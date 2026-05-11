use bitboard::Bitboard;
use piece::{Color, Piece, PieceType};
use square::Square;

mod bitboard;
mod piece;
mod square;

pub struct Board {
    pub bitboards: [[Bitboard; PieceType::COUNT]; Color::SIDES],
    pub mailbox: [Piece; Square::TOTAL as usize],
}
