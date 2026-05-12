use crate::board::{
    Board,
    piece::{Color, PieceType},
    square::Square,
};

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub struct Zobrist {
    pub pieces: [[[u64; Square::COUNT]; PieceType::COUNT]; Color::COUNT], // 64 squares * 6 piece types * 2 colors
    pub castling: [u64; 16],               // 16 castling rights (KQkq)
    pub en_passant: [u64; Square::COUNT], // 64 en passant squares (only 16 are valid but we can ignore the rest)
    pub side_to_move: [u64; Color::COUNT], // 0 for white to move, 1 for black to move
}

impl Zobrist {
    pub fn new() -> Self {
        const EMPTY: u64 = 0;
        const RNG_SEED: [u8; 32] = [248; 32];
        let mut random = ChaCha20Rng::from_seed(RNG_SEED); // Portable CSPRNG

        let mut zobrist = Self {
            pieces: [[[EMPTY; Square::COUNT]; PieceType::COUNT]; Color::COUNT],
            castling: [EMPTY; 16],
            en_passant: [EMPTY; Square::COUNT],
            side_to_move: [EMPTY; Color::COUNT],
        };

        for color in 0..Color::COUNT {
            for piece_type in 0..PieceType::COUNT {
                for square in 0..Square::COUNT {
                    zobrist.pieces[color][piece_type][square] = random.random::<u64>();
                }
            }
        }

        for i in 0..16 {
            zobrist.castling[i] = random.random::<u64>();
        }

        for square in 0..Square::COUNT {
            zobrist.en_passant[square] = random.random::<u64>();
        }

        for color in 0..Color::COUNT {
            zobrist.side_to_move[color] = random.random::<u64>();
        }

        zobrist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist() {
        let zobrist = Zobrist::new();
        assert_eq!(zobrist.pieces[0][0][0], 1432220013574475785);
        assert_eq!(zobrist.castling[0], 4926799129666148837);
        assert_eq!(zobrist.en_passant[0], 18043349099285482400);
        assert_eq!(zobrist.side_to_move, 14831281805493032206);
    }
}
