use bitboard::Bitboard;
use piece::{Color, Piece, PieceType};
use square::Square;
use zobrist::Zobrist;

mod bitboard;
mod piece;
mod square;
mod zobrist;

pub struct Board {
    pub bitboards: [[Bitboard; PieceType::COUNT]; Color::COUNT],
    pub mailbox: [Piece; Square::COUNT],
    pub zobrist: Zobrist,
}
