use crate::board::{bitboard::Bitboard, piece::Color, square::Square};

mod attack;
mod directions;

pub struct MoveGenerator {
    pub king_moves: [Bitboard; Square::COUNT],
    pub knight_moves: [Bitboard; Square::COUNT],
    pub pawn_attacks: [[Bitboard; Square::COUNT]; Color::COUNT],
    pub rook_moves: Vec<Bitboard>,
    pub bishop_moves: Vec<Bitboard>,
}

impl MoveGenerator {
    pub fn new() -> Self {
        let mut movegen = Self {
            king_moves: [Bitboard(0); Square::COUNT],
            knight_moves: [Bitboard(0); Square::COUNT],
            pawn_attacks: [[Bitboard(0); Square::COUNT]; Color::COUNT],
            rook_moves: Vec::new(),
            bishop_moves: Vec::new(),
        };

        for square in 0..Square::COUNT {
            movegen.init_pawn_attacks(Square::from_idx(square), Color::White);
            movegen.init_pawn_attacks(Square::from_idx(square), Color::Black);
            movegen.init_king_moves(Square::from_idx(square));
            movegen.init_knight_moves(Square::from_idx(square));
        }

        movegen
    }
}
