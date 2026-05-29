use arrayvec::ArrayVec;

use crate::{
    board::{
        Board,
        piece::{Color, Piece},
    },
    evaluation::piece_value,
    movegen::moves::{Move, MoveType},
};

// https://www.chessprogramming.org/Encoding_Moves#Move_Index
const MAX_MOVES: usize = 256;
const TT_MOVE_SCORE: i16 = 30_000;

#[derive(Copy, Clone, Debug, PartialEq)]
struct ScoredMove {
    m: Move,
    score: i16,
}

pub struct MoveList(ArrayVec<ScoredMove, MAX_MOVES>);

impl Default for MoveList {
    fn default() -> Self {
        Self(ArrayVec::new())
    }
}

impl MoveList {
    pub fn push(&mut self, m: Move) {
        self.0.push(ScoredMove { m, score: 0 });
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, idx: usize) -> Move {
        self.0[idx].m
    }

    pub fn score_moves(&mut self, board: &Board, tt_move: Option<Move>) {
        for scored in &mut self.0 {
            scored.score = if Some(scored.m) == tt_move {
                TT_MOVE_SCORE
            } else {
                score_move(board, scored.m)
            };
        }
    }

    pub fn pick_next(&mut self, start_idx: usize) -> Move {
        self.pick_next_scored(start_idx).0
    }

    pub fn pick_next_scored(&mut self, start_idx: usize) -> (Move, i16) {
        let mut best_idx = start_idx;

        for idx in start_idx + 1..self.0.len() {
            if self.0[idx].score > self.0[best_idx].score {
                best_idx = idx;
            }
        }

        self.0.swap(start_idx, best_idx);
        (self.0[start_idx].m, self.0[start_idx].score)
    }
}

fn score_move(board: &Board, m: Move) -> i16 {
    let promotion_score = match m.kind() {
        MoveType::QPromotion | MoveType::QPromoCapture => 9_000,
        MoveType::RPromotion | MoveType::RPromoCapture => 8_000,
        MoveType::BPromotion | MoveType::BPromoCapture => 7_000,
        MoveType::NPromotion | MoveType::NPromoCapture => 6_000,
        _ => 0,
    };

    if !m.is_capture() {
        return promotion_score;
    }

    let attacker = board.get_piece_at(m.from());
    let victim = match m.kind() {
        MoveType::EnPassant => match board.us() {
            Color::White => Piece::BlackPawn,
            Color::Black => Piece::WhitePawn,
        },
        _ => board.get_piece_at(m.to()),
    };

    let mvv_lva = piece_value(victim) as i16 * 10 - piece_value(attacker) as i16;
    10_000 + mvv_lva + promotion_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{Board, square::Square},
        movegen::moves::{Move, MoveType},
    };

    #[test]
    fn queen_capture_scores_above_pawn_capture() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1R4/8/8/4K3 w - - 0 1").unwrap();
        let queen_capture = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let pawn_capture = Move::new(Square::B4, Square::C5, MoveType::Capture);

        assert!(score_move(&board, queen_capture) > score_move(&board, pawn_capture));
    }

    #[test]
    fn pawn_capturing_queen_scores_above_queen_capturing_pawn() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1P4/8/8/4K3 w - - 0 1").unwrap();
        let pawn_captures_queen = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let queen_captures_pawn = Move::new(Square::B4, Square::C5, MoveType::Capture);

        assert!(score_move(&board, pawn_captures_queen) > score_move(&board, queen_captures_pawn));
    }

    #[test]
    fn queen_promotion_scores_above_quiet_move() {
        let board = Board::from_fen("4k3/4P3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let promotion = Move::new(Square::E7, Square::E8, MoveType::QPromotion);
        let quiet = Move::new(Square::E1, Square::D1, MoveType::Quiet);

        assert!(score_move(&board, promotion) > score_move(&board, quiet));
    }

    #[test]
    fn en_passant_scores_like_a_capture() {
        let board = Board::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let en_passant = Move::new(Square::E5, Square::D6, MoveType::EnPassant);
        let quiet = Move::new(Square::E1, Square::D1, MoveType::Quiet);

        assert!(score_move(&board, en_passant) > score_move(&board, quiet));
    }

    #[test]
    fn pick_next_selects_highest_scored_remaining_move() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1P4/8/8/4K3 w - - 0 1").unwrap();
        let quiet = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let queen_capture = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let pawn_capture = Move::new(Square::B4, Square::C5, MoveType::Capture);
        let mut moves = MoveList::default();

        moves.push(quiet);
        moves.push(pawn_capture);
        moves.push(queen_capture);
        moves.score_moves(&board, None);

        assert_eq!(moves.pick_next(0), queen_capture);
        assert_eq!(moves.pick_next(1), pawn_capture);
        assert_eq!(moves.pick_next(2), quiet);
    }

    #[test]
    fn pick_next_scored_returns_selected_move_score() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1P4/8/8/4K3 w - - 0 1").unwrap();
        let quiet = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let queen_capture = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let mut moves = MoveList::default();

        moves.push(quiet);
        moves.push(queen_capture);
        moves.score_moves(&board, None);

        let (m, score) = moves.pick_next_scored(0);

        assert_eq!(m, queen_capture);
        assert!(score > 0);
    }
}
