use crate::{
    board::{bitboard::Bitboard, piece::Color, square::Square},
    movegen::{
        directions::{KING_SHIFTS, KNIGHT_SHIFTS, MoveShift, PAWN_SHIFT_BLACK, PAWN_SHIFT_WHITE},
        magic::Magic,
    },
};

mod directions;
pub mod magic;

pub struct MoveGenerator {
    king_moves: [Bitboard; Square::COUNT],
    knight_moves: [Bitboard; Square::COUNT],
    pawn_attacks: [[Bitboard; Square::COUNT]; Color::COUNT],
    rook_moves: Vec<Bitboard>,
    bishop_moves: Vec<Bitboard>,
    // rook_magics: [Magic; Square::COUNT],
    // bishop_magics: [Magic; Square::COUNT],
}

#[derive(Copy, Clone)]
pub enum Slider {
    Rook,
    Bishop,
}

pub const ROOK_DIRS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
pub const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

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

    fn init_pawn_attacks(&mut self, square: Square, color: Color) {
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

    fn init_king_moves(&mut self, square: Square) {
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

    fn init_knight_moves(&mut self, square: Square) {
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

    pub fn rook_mask(square: Square) -> Bitboard {
        let mut mask = Bitboard(0);
        let file = square.file();
        let rank = square.rank();
        mask.set_file(file);
        mask.set_rank(rank);
        mask.clear(square);
        mask &= !mask.edges_excluding_square(square);

        mask
    }

    pub fn blocker_boards(mask: Bitboard) -> Vec<Bitboard> {
        let mut blockers = Vec::new();
        let mut n = 0u64;
        let d = mask.0;

        // Generate all subsets of a set
        // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
        loop {
            blockers.push(Bitboard(n));
            n = (n.wrapping_sub(d)) & d;
            if n == 0 {
                break;
            }
        }

        blockers
    }

    pub fn init_rook_magics(&mut self) {
        let square = 0;
        let mask = Self::rook_mask(Square::from_idx(square));
        // let blockers = 2u64.pow(mask.count_ones() as u32);
    }

    pub fn slider_moves(square: Square, blockers: Bitboard, slider: Slider) -> Bitboard {
        let mut moves = Bitboard(0);
        let dirs = match slider {
            Slider::Rook => ROOK_DIRS,
            Slider::Bishop => BISHOP_DIRS,
        };

        for (df, dr) in dirs {
            let mut ray = square;
            while !blockers.contains(ray) {
                ray = match ray.try_offset(df, dr) {
                    Some(sq) => sq,
                    None => break,
                };
                moves.set(ray);
            }
        }

        moves
    }

    pub fn magic_index(occupancy: Bitboard, magic: &Magic) -> usize {
        let blockers = occupancy & magic.mask;
        (blockers.0.wrapping_mul(magic.magic) >> magic.shift) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rook_mask() {
        for s in 0..Square::COUNT {
            let square = Square::from_idx(s);
            let mask = MoveGenerator::rook_mask(square);
            println!("Rook mask for square {}: \n{}", square, mask);
        }
    }

    #[test]
    fn test_rook_moves() {
        let square = Square::A1;
        let blockers = Bitboard::from_square(Square::A4);
        let moves = MoveGenerator::slider_moves(square, blockers, Slider::Rook);
        println!(
            "Rook moves for square {} with blockers {}: \n{}",
            square, blockers, moves
        );

        let square = Square::D4;
        let mut blockers = Bitboard::from_square(Square::D7);
        blockers.set(Square::D3);
        blockers.set(Square::D7);
        blockers.set(Square::H4);
        let moves = MoveGenerator::slider_moves(square, blockers, Slider::Rook);
        println!(
            "Rook moves for square {} with blockers {}: \n{}",
            square, blockers, moves
        );
    }
}
