use crate::board::{
    Board,
    piece::{Color, Piece, PieceType},
};

pub type Score = i16;

// Larry Kaufman centipawn scale
// https://www.chessprogramming.org/Point_Value
pub const PIECE_VALUES: [u16; PieceType::COUNT] = [100, 350, 350, 525, 1000, 0];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Evaluation {
    material: [u16; Color::COUNT],
}

impl Evaluation {
    pub fn new(board: &Board) -> Self {
        Self {
            material: Self::count_material(board),
        }
    }

    pub fn material(&self) -> [u16; Color::COUNT] {
        self.material
    }

    pub fn refresh(&mut self, board: &Board) {
        self.material = Self::count_material(board);
    }

    pub fn add_piece(&mut self, piece: Piece) {
        self.material[piece.color()] += piece_value(piece);
    }

    pub fn remove_piece(&mut self, piece: Piece) {
        self.material[piece.color()] -= piece_value(piece);
    }

    pub fn score(&self, side: Color) -> Score {
        let us = self.material[side] as Score;
        let them = self.material[!side] as Score;

        us - them
    }

    pub fn count_material(board: &Board) -> [u16; Color::COUNT] {
        let mut material = [0; Color::COUNT];

        for color in [Color::White, Color::Black] {
            for piece_type in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                let count = board.bitboards[color][piece_type].count_ones();
                material[color] += count as u16 * PIECE_VALUES[piece_type];
            }
        }

        material
    }
}

pub fn piece_value(piece: Piece) -> u16 {
    match piece {
        Piece::None => 0,
        _ => PIECE_VALUES[piece.ptype()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::square::Square,
        movegen::{
            MoveGenerator,
            moves::{Move, MoveType},
        },
    };

    #[test]
    fn counts_starting_material() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        assert_eq!(Evaluation::count_material(&board), [4250, 4250]);
        assert_eq!(board.state.evaluation.material(), [4250, 4250]);
    }

    #[test]
    fn tracks_material_through_make_and_unmake() {
        let mut board = Board::from_fen("8/8/8/3p4/4P3/8/8/4K2k w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let capture = Move::new(Square::E4, Square::D5, MoveType::Capture);

        assert!(board.make(capture, &movegen));
        assert_eq!(board.state.evaluation.material(), [100, 0]);

        board.unmake();
        assert_eq!(board.state.evaluation.material(), [100, 100]);
    }
}
