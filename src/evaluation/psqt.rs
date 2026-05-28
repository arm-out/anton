use crate::board::{piece::PieceType, square::Square};

// Larry Kaufman centipawn scale
// https://www.chessprogramming.org/Point_Value
pub const PIECE_VALUES: [i32; PieceType::COUNT] = [100, 350, 350, 525, 1000, 0];

// Values from Sebastian Lague
// https://github.com/SebLague/Chess-Coding-Adventure
#[rustfmt::skip]
const PAWNS_MG: [i32; Square::COUNT] = [
     0, 0,  0,  0,  0,  0,  0,  0,
    50,50, 50, 50, 50, 50, 50, 50,
    10,10, 20, 30, 30, 20, 10, 10,
     5, 5, 10, 25, 25, 10,  5,  5,
     0, 0,  0, 20, 20,  0,  0,  0,
     5,-5,-10,  0,  0,-10, -5,  5,
     5,10, 10,-20,-20, 10, 10,  5,
     0, 0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const PAWNS_EG: [i32; Square::COUNT] = [
      0,  0,  0,  0,  0,  0,  0,  0,
     80, 80, 80, 80, 80, 80, 80, 80,
     50, 50, 50, 50, 50, 50, 50, 50,
     30, 30, 30, 30, 30, 30, 30, 30,
     20, 20, 20, 20, 20, 20, 20, 20,
     10, 10, 10, 10, 10, 10, 10, 10,
     10, 10, 10, 10, 10, 10, 10, 10,
      0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const KNIGHTS: [i32; Square::COUNT] = [
     -50,-40,-30,-30,-30,-30,-40,-50,
     -40,-20,  0,  0,  0,  0,-20,-40,
     -30,  0, 10, 15, 15, 10,  0,-30,
     -30,  5, 15, 20, 20, 15,  5,-30,
     -30,  0, 15, 20, 20, 15,  0,-30,
     -30,  5, 10, 15, 15, 10,  5,-30,
     -40,-20,  0,  5,  5,  0,-20,-40,
     -50,-40,-30,-30,-30,-30,-40,-50
];

#[rustfmt::skip]
const BISHOPS: [i32; Square::COUNT] = [
     -20,-10,-10,-10,-10,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5, 10, 10,  5,  0,-10,
     -10,  5,  5, 10, 10,  5,  5,-10,
     -10,  0, 10, 10, 10, 10,  0,-10,
     -10, 10, 10, 10, 10, 10, 10,-10,
     -10,  5,  0,  0,  0,  0,  5,-10,
     -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const ROOKS: [i32; Square::COUNT] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0
];

#[rustfmt::skip]
const QUEENS: [i32; Square::COUNT] = [
     -20,-10,-10, -5, -5,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5,  5,  5,  5,  0,-10,
      -5,  0,  5,  5,  5,  5,  0, -5,
      0,   0,  5,  5,  5,  5,  0, -5,
     -10,  5,  5,  5,  5,  5,  0,-10,
     -10,  0,  5,  0,  0,  0,  0,-10,
     -20,-10,-10, -5, -5,-10,-10,-20
];

#[rustfmt::skip]
const KING_MG: [i32; Square::COUNT] = [
     -80, -70, -70, -70, -70, -70, -70, -80, 
     -60, -60, -60, -60, -60, -60, -60, -60, 
     -40, -50, -50, -60, -60, -50, -50, -40, 
     -30, -40, -40, -50, -50, -40, -40, -30, 
     -20, -30, -30, -40, -40, -30, -30, -20, 
     -10, -20, -20, -20, -20, -20, -20, -10, 
     20,  20,  -5,  -5,  -5,  -5,  20,  20, 
     20,  30,  10,   0,   0,  10,  30,  20
];

#[rustfmt::skip]
const KING_EG: [i32; Square::COUNT] = [
     -20, -10, -10, -10, -10, -10, -10, -20,
     -5,   0,   5,   5,   5,   5,   0,  -5,
     -10, -5,   20,  30,  30,  20,  -5, -10,
     -15, -10,  35,  45,  45,  35, -10, -15,
     -20, -15,  30,  40,  40,  30, -15, -20,
     -25, -20,  20,  25,  25,  20, -20, -25,
     -30, -25,   0,   0,   0,   0, -25, -30,
     -50, -30, -30, -30, -30, -30, -30, -50
];

pub const PAWN_MG_PSQT: [i32; Square::COUNT] = add_piece_value(PAWNS_MG, PieceType::Pawn);
pub const PAWN_EG_PSQT: [i32; Square::COUNT] = add_piece_value(PAWNS_EG, PieceType::Pawn);
pub const KNIGHT_PSQT: [i32; Square::COUNT] = add_piece_value(KNIGHTS, PieceType::Knight);
pub const BISHOP_PSQT: [i32; Square::COUNT] = add_piece_value(BISHOPS, PieceType::Bishop);
pub const ROOK_PSQT: [i32; Square::COUNT] = add_piece_value(ROOKS, PieceType::Rook);
pub const QUEEN_PSQT: [i32; Square::COUNT] = add_piece_value(QUEENS, PieceType::Queen);
pub const KING_MG_PSQT: [i32; Square::COUNT] = add_piece_value(KING_MG, PieceType::King);
pub const KING_EG_PSQT: [i32; Square::COUNT] = add_piece_value(KING_EG, PieceType::King);

const fn add_piece_value(
    table: [i32; Square::COUNT],
    piece_type: PieceType,
) -> [i32; Square::COUNT] {
    let mut result = [0; Square::COUNT];
    let piece_value = PIECE_VALUES[piece_type as usize];
    let mut i = 0;

    while i < Square::COUNT {
        result[i] = table[i] + piece_value;
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::board::piece::Color;

    use super::*;

    #[test]
    fn test_psqt_idx() {
        assert_eq!(Square::A8.psqt_idx(Color::White), 0);
        assert_eq!(Square::H8.psqt_idx(Color::White), 7);
        assert_eq!(Square::A1.psqt_idx(Color::White), 56);
        assert_eq!(Square::H1.psqt_idx(Color::White), 63);
        assert_eq!(Square::E4.psqt_idx(Color::White), 36);

        assert_eq!(Square::A8.psqt_idx(Color::Black), 63);
        assert_eq!(Square::H8.psqt_idx(Color::Black), 56);
        assert_eq!(Square::A1.psqt_idx(Color::Black), 7);
        assert_eq!(Square::H1.psqt_idx(Color::Black), 0);
        assert_eq!(Square::E5.psqt_idx(Color::Black), 35);
    }
}
