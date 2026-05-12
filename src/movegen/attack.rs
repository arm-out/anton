use crate::{
    board::{bitboard::Bitboard, piece::Color, square::Square},
    movegen::{
        MoveGenerator,
        directions::{KING_SHIFTS, KNIGHT_SHIFTS, MoveShift, PAWN_SHIFT_BLACK, PAWN_SHIFT_WHITE},
    },
};

impl MoveGenerator {
    pub fn init_pawn_attacks(&mut self, square: Square, color: Color) {
        let mask = Bitboard::from_square(square);
        let shifts = match color {
            Color::White => PAWN_SHIFT_WHITE,
            Color::Black => PAWN_SHIFT_BLACK,
        };

        for MoveShift { shift, exclude } in shifts {
            let candidate = if color == Color::White {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };

            self.pawn_attacks[color][square] |= candidate;
        }
    }

    pub fn init_king_moves(&mut self, square: Square) {
        let mask = Bitboard::from_square(square);

        for MoveShift { shift, exclude } in KING_SHIFTS {
            let candidate = if shift > 0 {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };
            self.king_moves[square] |= candidate;
        }
    }

    pub fn init_knight_moves(&mut self, square: Square) {
        let mask = Bitboard::from_square(square);

        for MoveShift { shift, exclude } in KNIGHT_SHIFTS {
            let candidate = if shift > 0 {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };
            self.knight_moves[square] |= candidate;
        }
    }
}
