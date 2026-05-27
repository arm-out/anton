use crate::{
    board::{
        Board,
        piece::{Color, Piece, PieceType},
        square::Square,
    },
    evaluation::psqt::PIECE_VALUES,
};

mod psqt;

pub type Score = i16;

pub const MAX_GAME_PHASE: u8 = 24;
pub const PHASE_VALUES: [u8; PieceType::COUNT] = [0, 1, 1, 2, 4, 0];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Evaluation {
    mg: [Score; Color::COUNT],
    eg: [Score; Color::COUNT],
}

impl Evaluation {
    pub fn new(board: &Board) -> Self {
        let mut evaluation = Self::default();

        for (idx, piece) in board.mailbox.iter().enumerate() {
            if *piece != Piece::None {
                evaluation.add_piece(Square::from_idx(idx), *piece);
            }
        }

        evaluation
    }

    pub fn refresh(&mut self, board: &Board) {
        *self = Self::new(board);
    }

    pub fn add_piece(&mut self, square: Square, piece: Piece) {
        let color = piece.color();
        let (mg, eg) = psqt_value(square, piece);

        self.mg[color] += mg;
        self.eg[color] += eg;
    }

    pub fn remove_piece(&mut self, square: Square, piece: Piece) {
        let color = piece.color();
        let (mg, eg) = psqt_value(square, piece);

        self.mg[color] -= mg;
        self.eg[color] -= eg;
    }

    // https://www.chessprogramming.org/Tapered_Eval
    pub fn score(&self, side: Color, game_phase: u8) -> Score {
        let mg = (self.mg[side] - self.mg[!side]) as i32;
        let eg = (self.eg[side] - self.eg[!side]) as i32;
        let phase = game_phase.min(MAX_GAME_PHASE) as i32;
        let eg_phase = MAX_GAME_PHASE as i32 - phase;

        ((mg * phase + eg * eg_phase) / MAX_GAME_PHASE as i32) as Score
    }
}

pub fn psqt_value(square: Square, piece: Piece) -> (Score, Score) {
    let idx = square.psqt_idx(piece.color());
    match piece.ptype() {
        PieceType::Pawn => (
            psqt::PAWN_MG_PSQT[idx] as Score,
            psqt::PAWN_EG_PSQT[idx] as Score,
        ),
        PieceType::Knight => (
            psqt::KNIGHT_PSQT[idx] as Score,
            psqt::KNIGHT_PSQT[idx] as Score,
        ),
        PieceType::Bishop => (
            psqt::BISHOP_PSQT[idx] as Score,
            psqt::BISHOP_PSQT[idx] as Score,
        ),
        PieceType::Rook => (psqt::ROOK_PSQT[idx] as Score, psqt::ROOK_PSQT[idx] as Score),
        PieceType::Queen => (
            psqt::QUEEN_PSQT[idx] as Score,
            psqt::QUEEN_PSQT[idx] as Score,
        ),
        PieceType::King => (
            psqt::KING_MG_PSQT[idx] as Score,
            psqt::KING_EG_PSQT[idx] as Score,
        ),
        PieceType::None => (0, 0),
    }
}

pub fn piece_value(piece: Piece) -> u16 {
    match piece {
        Piece::None => 0,
        _ => PIECE_VALUES[piece.ptype()] as u16,
    }
}

pub fn phase_value(piece: Piece) -> u8 {
    match piece {
        Piece::None => 0,
        _ => PHASE_VALUES[piece.ptype()],
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
    fn initializes_starting_evaluation() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        assert_eq!(board.state.evaluation, Evaluation::new(&board));
        assert_eq!(
            board
                .state
                .evaluation
                .score(Color::White, board.state.game_phase),
            0
        );
        assert_eq!(board.state.game_phase, MAX_GAME_PHASE);
    }

    #[test]
    fn tracks_evaluation_through_make_and_unmake() {
        let mut board = Board::from_fen("8/8/8/3n4/4B3/8/8/4K2k w - - 0 1").unwrap();
        let initial_evaluation = board.state.evaluation;
        let movegen = MoveGenerator::new();
        let capture = Move::new(Square::E4, Square::D5, MoveType::Capture);

        assert_eq!(board.state.game_phase, 2);
        assert!(board.make(capture, &movegen));
        assert_eq!(board.state.evaluation, Evaluation::new(&board));
        assert_eq!(board.state.game_phase, 1);

        board.unmake();
        assert_eq!(board.state.evaluation, initial_evaluation);
        assert_eq!(board.state.evaluation, Evaluation::new(&board));
        assert_eq!(board.state.game_phase, 2);
    }

    #[test]
    fn tracks_game_phase_through_promotion_and_unmake() {
        let mut board = Board::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let promotion = Move::new(Square::A7, Square::A8, MoveType::QPromotion);

        assert_eq!(board.state.game_phase, 0);
        assert!(board.make(promotion, &movegen));
        assert_eq!(board.state.evaluation, Evaluation::new(&board));
        assert_eq!(board.state.game_phase, 4);

        board.unmake();
        assert_eq!(board.state.evaluation, Evaluation::new(&board));
        assert_eq!(board.state.game_phase, 0);
    }

    #[test]
    fn tracks_evaluation_through_castling_and_unmake() {
        let movegen = MoveGenerator::new();

        for (fen, from, to, kind) in [
            (
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                Square::E1,
                Square::G1,
                MoveType::CastleKingside,
            ),
            (
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                Square::E1,
                Square::C1,
                MoveType::CastleQueenside,
            ),
            (
                "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                Square::E8,
                Square::G8,
                MoveType::CastleKingside,
            ),
            (
                "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                Square::E8,
                Square::C8,
                MoveType::CastleQueenside,
            ),
        ] {
            let mut board = Board::from_fen(fen).unwrap();
            let initial_evaluation = board.state.evaluation;
            let initial_phase = board.state.game_phase;
            let castle = Move::new(from, to, kind);

            assert!(board.make(castle, &movegen));
            assert_eq!(board.state.evaluation, Evaluation::new(&board));
            assert_eq!(board.state.game_phase, initial_phase);

            board.unmake();
            assert_eq!(board.state.evaluation, initial_evaluation);
            assert_eq!(board.state.evaluation, Evaluation::new(&board));
            assert_eq!(board.state.game_phase, initial_phase);
        }
    }
}
