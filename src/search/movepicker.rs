use crate::{
    board::{
        Board,
        piece::{Color, Piece},
    },
    evaluation::piece_value,
    movegen::{
        Evasions, MoveGenerator, MoveList, Noisy, Quiet,
        moves::{Move, MoveType},
    },
};

#[derive(Copy, Clone)]
enum Stage {
    TTMove,
    GenCaptures,
    Captures,
    Quiets,
    TTEvasion,
    GenEvasions,
    Evasions,
    Done,
}

pub(super) struct MovePicker {
    stage: Stage,
    tt_move: Option<Move>,
    gen_quiets: bool,
    moves: MoveList,
    idx: usize,
}

impl MovePicker {
    pub(super) fn new(tt_move: Option<Move>, gen_quiets: bool) -> Self {
        Self {
            stage: Stage::TTMove,
            tt_move,
            gen_quiets,
            moves: MoveList::default(),
            idx: 0,
        }
    }

    pub(super) fn evasions(tt_move: Option<Move>) -> Self {
        Self {
            stage: Stage::TTEvasion,
            tt_move,
            gen_quiets: true,
            moves: MoveList::default(),
            idx: 0,
        }
    }

    pub(super) fn next_move(&mut self, board: &Board, movegen: &MoveGenerator) -> Option<Move> {
        loop {
            match self.stage {
                Stage::TTMove => {
                    self.stage = Stage::GenCaptures;
                    if let Some(m) = self.tt_move {
                        return Some(m);
                    }
                }
                Stage::GenCaptures => {
                    self.moves = movegen.gen_moves::<Noisy>(board);
                    score_moves(board, &mut self.moves);
                    self.idx = 0;
                    self.stage = Stage::Captures;
                }
                Stage::Captures => {
                    if let Some(m) = self.pick_next() {
                        return Some(m);
                    }

                    self.stage = Stage::Quiets;
                }
                Stage::Quiets => {
                    if self.gen_quiets {
                        self.moves = movegen.gen_moves::<Quiet>(board);
                        self.idx = 0;
                        self.gen_quiets = false;
                    }

                    if let Some(m) = self.pick_next() {
                        return Some(m);
                    }

                    self.stage = Stage::Done;
                }
                Stage::TTEvasion => {
                    self.stage = Stage::GenEvasions;
                    if let Some(m) = self.tt_move {
                        return Some(m);
                    }
                }
                Stage::GenEvasions => {
                    self.moves = movegen.gen_moves::<Evasions>(board);
                    score_moves(board, &mut self.moves);
                    self.idx = 0;
                    self.stage = Stage::Evasions;
                }
                Stage::Evasions => {
                    if let Some(m) = self.pick_next() {
                        return Some(m);
                    }

                    self.stage = Stage::Done;
                }
                Stage::Done => return None,
            }
        }
    }

    fn pick_next(&mut self) -> Option<Move> {
        while self.idx < self.moves.len() {
            let m = self.moves.pick_next(self.idx);
            self.idx += 1;

            if Some(m) == self.tt_move {
                continue;
            }

            return Some(m);
        }

        None
    }
}

fn score_moves(board: &Board, moves: &mut MoveList) {
    for scored in moves.iter_mut() {
        scored.score = score_move(board, scored.m);
    }
}

fn score_move(board: &Board, m: Move) -> i16 {
    let promotion_score = promotion_score(m);

    if !m.is_capture() {
        return promotion_score;
    }

    let attacker = board.get_piece_at(m.from());
    let victim = capture_victim(board, m);
    let mvv_lva = piece_value(victim) as i16 * 10 - piece_value(attacker) as i16;

    10_000 + mvv_lva + promotion_score
}

fn promotion_score(m: Move) -> i16 {
    match m.kind() {
        MoveType::QPromotion | MoveType::QPromoCapture => 9_000,
        MoveType::RPromotion | MoveType::RPromoCapture => 8_000,
        MoveType::BPromotion | MoveType::BPromoCapture => 7_000,
        MoveType::NPromotion | MoveType::NPromoCapture => 6_000,
        _ => 0,
    }
}

fn capture_victim(board: &Board, m: Move) -> Piece {
    match m.kind() {
        MoveType::EnPassant => match board.us() {
            Color::White => Piece::BlackPawn,
            Color::Black => Piece::WhitePawn,
        },
        _ => board.get_piece_at(m.to()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::Square;

    #[test]
    fn tt_move_is_returned_first() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E4, Square::E5, MoveType::Capture);
        let mut picker = MovePicker::new(Some(tt_move), true);

        assert_eq!(picker.next_move(&board, &movegen), Some(tt_move));
    }

    #[test]
    fn tt_move_is_not_returned_twice() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E4, Square::E5, MoveType::Capture);
        let mut picker = MovePicker::new(Some(tt_move), true);
        let mut tt_count = 0;

        while let Some(m) = picker.next_move(&board, &movegen) {
            if m == tt_move {
                tt_count += 1;
            }
        }

        assert_eq!(tt_count, 1);
    }

    #[test]
    fn captures_are_returned_before_quiets() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let mut picker = MovePicker::new(None, true);
        let mut seen_quiet = false;

        while let Some(m) = picker.next_move(&board, &movegen) {
            if m.is_capture() || promotion_score(m) > 0 {
                assert!(!seen_quiet);
            } else {
                seen_quiet = true;
            }
        }

        assert!(seen_quiet);
    }

    #[test]
    fn main_mode_returns_all_noisy_and_quiet_moves() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let noisy = movegen.gen_moves::<Noisy>(&board);
        let quiet = movegen.gen_moves::<Quiet>(&board);
        let mut picker = MovePicker::new(None, true);
        let mut count = 0;

        while picker.next_move(&board, &movegen).is_some() {
            count += 1;
        }

        assert_eq!(count, noisy.len() + quiet.len());
    }

    #[test]
    fn quiets_can_be_skipped() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let mut picker = MovePicker::new(None, false);

        while let Some(m) = picker.next_move(&board, &movegen) {
            assert!(m.is_capture() || promotion_score(m) > 0);
        }
    }

    #[test]
    fn evasion_mode_returns_tt_evasion_first() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4q3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E1, Square::E2, MoveType::Capture);
        let mut picker = MovePicker::evasions(Some(tt_move));

        assert_eq!(picker.next_move(&board, &movegen), Some(tt_move));
    }

    #[test]
    fn evasion_mode_returns_generated_evasions() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4q3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let evasions = movegen.gen_moves::<Evasions>(&board);
        let mut picker = MovePicker::evasions(None);
        let mut count = 0;

        while picker.next_move(&board, &movegen).is_some() {
            count += 1;
        }

        assert_eq!(count, evasions.len());
    }

    #[test]
    fn mvv_lva_prefers_higher_value_victim() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1R4/8/8/4K3 w - - 0 1").unwrap();
        let queen_capture = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let pawn_capture = Move::new(Square::B4, Square::C5, MoveType::Capture);

        assert!(score_move(&board, queen_capture) > score_move(&board, pawn_capture));
    }

    #[test]
    fn mvv_lva_prefers_lower_value_attacker() {
        let board = Board::from_fen("4k3/8/8/2p1q3/1Q1P4/8/8/4K3 w - - 0 1").unwrap();
        let pawn_captures_queen = Move::new(Square::D4, Square::E5, MoveType::Capture);
        let queen_captures_pawn = Move::new(Square::B4, Square::C5, MoveType::Capture);

        assert!(score_move(&board, pawn_captures_queen) > score_move(&board, queen_captures_pawn));
    }

    #[test]
    fn en_passant_scores_like_a_capture() {
        let board = Board::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let en_passant = Move::new(Square::E5, Square::D6, MoveType::EnPassant);
        let quiet = Move::new(Square::E1, Square::D1, MoveType::Quiet);

        assert!(score_move(&board, en_passant) > score_move(&board, quiet));
    }
}
