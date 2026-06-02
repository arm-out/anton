use crate::board::{
    Board,
    piece::{Color, Piece, PieceType},
    square::Square,
};

pub mod params;
pub mod trace;
mod weights;

pub use params::{EVAL_PARAMS, EvalContext, EvalParams, EvalScore};
pub use trace::{EvalTerm, FeatureVectorTrace, NoTrace, Trace};

pub type Score = i16;
pub type EvalValue = i32;

pub const MAX_GAME_PHASE: u8 = 24;
pub const PHASE_VALUES: [u8; PieceType::COUNT] = [0, 1, 1, 2, 4, 0];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Evaluation {
    scores: [EvalScore; Color::COUNT],
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

        self.scores[color] += psqt_value(square, piece);
    }

    pub fn remove_piece(&mut self, square: Square, piece: Piece) {
        let color = piece.color();

        self.scores[color] -= psqt_value(square, piece);
    }

    pub fn score(&self, board: &Board) -> Score {
        let eval = self.scores[Color::White] - self.scores[Color::Black];
        let score = eval.tapered(board.state.game_phase);

        match board.us() {
            Color::White => score,
            Color::Black => -score,
        }
    }
}

pub fn evaluate<T: Trace>(board: &Board, params: &EvalParams, trace: &mut T) -> Score {
    if T::USE_INCREMENTAL_EVAL && *params == EVAL_PARAMS {
        return board.state.evaluation.score(board);
    }

    let mut ctx = EvalContext;
    let eval = evaluate_side(board, &mut ctx, Color::White, params, trace)
        - evaluate_side(board, &mut ctx, Color::Black, params, trace);
    let score = eval.tapered(board.state.game_phase);

    match board.us() {
        Color::White => score,
        Color::Black => -score,
    }
}

pub fn evaluate_static(board: &Board) -> Score {
    let mut trace = NoTrace;
    evaluate(board, &EVAL_PARAMS, &mut trace)
}

fn evaluate_side<T: Trace>(
    board: &Board,
    _ctx: &mut EvalContext,
    side: Color,
    params: &EvalParams,
    trace: &mut T,
) -> EvalScore {
    let mut eval = EvalScore::zero();

    for square in board.occupancy[side] {
        let piece = board.get_piece_at(square);
        let ptype = piece.ptype();
        let psqt_idx = square.psqt_idx(side);

        add_material(&mut eval, trace, side, ptype, params.material[ptype]);
        add_psqt(
            &mut eval,
            trace,
            side,
            ptype,
            psqt_idx,
            params.psqt[ptype][psqt_idx],
        );
    }

    eval
}

fn add_material(
    eval: &mut EvalScore,
    trace: &mut impl Trace,
    side: Color,
    ptype: PieceType,
    value: EvalScore,
) {
    *eval += value;
    trace.term(side, EvalTerm::Material(ptype), 1);
}

fn add_psqt(
    eval: &mut EvalScore,
    trace: &mut impl Trace,
    side: Color,
    ptype: PieceType,
    psqt_idx: usize,
    value: EvalScore,
) {
    *eval += value;
    trace.term(side, EvalTerm::Psqt(ptype, psqt_idx), 1);
}

pub fn psqt_value(square: Square, piece: Piece) -> EvalScore {
    EVAL_PARAMS.piece_square_value(square, piece)
}

pub fn piece_value(piece: Piece) -> u16 {
    match piece {
        Piece::None => 0,
        _ => EVAL_PARAMS.material[piece.ptype()].mg as u16,
    }
}

pub fn phase_value(piece: Piece) -> u8 {
    match piece {
        Piece::None => 0,
        _ => PHASE_VALUES[piece.ptype()],
    }
}

pub(crate) fn eval_to_score(eval: EvalValue) -> Score {
    eval.clamp(Score::MIN as EvalValue, Score::MAX as EvalValue) as Score
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
        let mut trace = NoTrace;
        assert_eq!(
            evaluate_static(&board),
            evaluate(&board, &EVAL_PARAMS, &mut trace)
        );
        assert_eq!(board.state.game_phase, MAX_GAME_PHASE);
    }

    #[test]
    fn evaluation_scores_material_advantage() {
        let white_queen = Board::from_fen("4k3/8/8/8/8/8/8/4KQ2 w - - 0 1").unwrap();
        let black_queen = Board::from_fen("4kq2/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();

        assert!(evaluate_static(&white_queen) > 900);
        assert!(evaluate_static(&black_queen) < -900);
    }

    #[test]
    fn parameterized_eval_matches_incremental_eval() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "4k3/8/8/8/8/8/8/4KQ2 w - - 0 1",
            "4kq2/8/8/8/8/8/8/4K3 w - - 0 1",
            "8/8/8/3n4/4B3/8/8/4K2k b - - 0 1",
        ] {
            let board = Board::from_fen(fen).unwrap();
            let mut trace = NoTrace;

            assert_eq!(
                evaluate(&board, &EVAL_PARAMS, &mut trace),
                evaluate_static(&board)
            );
        }
    }

    #[test]
    fn no_trace_uses_custom_params_when_params_are_not_current() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KQ2 w - - 0 1").unwrap();
        let mut weights = EVAL_PARAMS.weights();
        weights[params::material_mg_weight_idx(PieceType::Queen as usize)] += 100;
        weights[params::material_eg_weight_idx(PieceType::Queen as usize)] += 100;
        let params = EvalParams::from_weights(weights);
        let mut trace = NoTrace;

        assert_ne!(
            evaluate(&board, &params, &mut trace),
            evaluate_static(&board)
        );
    }

    #[test]
    fn feature_trace_still_rebuilds_features() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KQ2 w - - 0 1").unwrap();
        let mut trace = FeatureVectorTrace::new();

        evaluate(&board, &EVAL_PARAMS, &mut trace);

        assert_eq!(trace.material_feature(PieceType::Queen), 1);
    }

    #[test]
    fn trace_records_lone_white_queen_features() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KQ2 w - - 0 1").unwrap();
        let mut trace = FeatureVectorTrace::new();

        evaluate(&board, &EVAL_PARAMS, &mut trace);

        assert_eq!(trace.material_feature(PieceType::Queen), 1);
        assert_eq!(
            trace.psqt_feature(PieceType::Queen, Square::F1.psqt_idx(Color::White)),
            1
        );
        assert_eq!(trace.material_feature(PieceType::King), 0);
    }

    #[test]
    fn trace_records_mirrored_black_piece_with_opposite_sign() {
        let board = Board::from_fen("4kq2/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let mut trace = FeatureVectorTrace::new();

        evaluate(&board, &EVAL_PARAMS, &mut trace);

        assert_eq!(trace.material_feature(PieceType::Queen), -1);
        assert_eq!(
            trace.psqt_feature(PieceType::Queen, Square::F8.psqt_idx(Color::Black)),
            -1
        );
        assert_eq!(trace.material_feature(PieceType::King), 0);
    }

    #[test]
    fn tapered_trace_features_match_white_pov_eval() {
        let board = Board::from_fen("4kq2/8/8/8/8/8/8/4KQ2 w - - 0 1").unwrap();
        let mut trace = FeatureVectorTrace::new();
        let mut no_trace = NoTrace;
        let mut ctx = EvalContext;

        evaluate(&board, &EVAL_PARAMS, &mut trace);

        let eval = evaluate_side(&board, &mut ctx, Color::White, &EVAL_PARAMS, &mut no_trace)
            - evaluate_side(&board, &mut ctx, Color::Black, &EVAL_PARAMS, &mut no_trace);
        let phase = board.state.game_phase.min(MAX_GAME_PHASE) as f64;
        let expected = (eval.mg as f64 * phase + eval.eg as f64 * (MAX_GAME_PHASE as f64 - phase))
            / MAX_GAME_PHASE as f64;
        let weights = EVAL_PARAMS.weights();
        let actual = trace
            .tapered_features(board.state.game_phase)
            .iter()
            .zip(weights)
            .map(|(feature, weight)| feature * weight as f64)
            .sum::<f64>();

        assert!((actual - expected).abs() < 1e-9);
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
