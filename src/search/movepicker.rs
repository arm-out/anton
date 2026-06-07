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

use super::ButterflyHistory;

#[derive(Copy, Clone)]
enum Stage {
    TTMove,
    GenCaptures,
    Captures,
    Killer1,
    Killer2,
    Quiets,
    TTEvasion,
    GenEvasions,
    Evasions,
    Done,
}

pub(super) struct MovePicker {
    stage: Stage,
    tt_move: Option<Move>,
    killers: [Option<Move>; 2],
    returned_killers: [bool; 2],
    gen_quiets: bool,
    moves: MoveList,
    idx: usize,
}

impl MovePicker {
    pub(super) fn new(tt_move: Option<Move>, killers: [Option<Move>; 2], gen_quiets: bool) -> Self {
        Self {
            stage: Stage::TTMove,
            tt_move,
            killers,
            returned_killers: [false; 2],
            gen_quiets,
            moves: MoveList::default(),
            idx: 0,
        }
    }

    pub(super) fn evasions(tt_move: Option<Move>) -> Self {
        Self {
            stage: Stage::TTEvasion,
            tt_move,
            killers: [None, None],
            returned_killers: [false; 2],
            gen_quiets: true,
            moves: MoveList::default(),
            idx: 0,
        }
    }

    pub(super) fn next_move(
        &mut self,
        board: &Board,
        movegen: &MoveGenerator,
        history: &ButterflyHistory,
    ) -> Option<Move> {
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
                    score_noisy(board, &mut self.moves);
                    self.idx = 0;
                    self.stage = Stage::Captures;
                }
                Stage::Captures => {
                    if let Some(m) = self.pick_next() {
                        return Some(m);
                    }

                    self.stage = Stage::Killer1;
                }
                Stage::Killer1 => {
                    self.stage = Stage::Killer2;

                    if self.gen_quiets
                        && let Some(m) = self.killers[0]
                        && self.should_return_killer(m, 0)
                        && movegen.is_pseudolegal_killer(board, m)
                    {
                        self.returned_killers[0] = true;
                        return Some(m);
                    }
                }
                Stage::Killer2 => {
                    self.stage = Stage::Quiets;

                    if self.gen_quiets
                        && let Some(m) = self.killers[1]
                        && self.should_return_killer(m, 1)
                        && movegen.is_pseudolegal_killer(board, m)
                    {
                        self.returned_killers[1] = true;
                        return Some(m);
                    }
                }
                Stage::Quiets => {
                    if self.gen_quiets {
                        self.moves = movegen.gen_moves::<Quiet>(board);
                        score_quiets(board, history, &mut self.moves);
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
                    score_noisy(board, &mut self.moves);
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

            if matches!(self.stage, Stage::Quiets) && self.returned_killer(m) {
                continue;
            }

            return Some(m);
        }

        None
    }

    fn should_return_killer(&self, m: Move, slot: usize) -> bool {
        Some(m) != self.tt_move && (slot == 0 || Some(m) != self.killers[0])
    }

    fn returned_killer(&self, m: Move) -> bool {
        for slot in 0..self.killers.len() {
            if self.returned_killers[slot] && self.killers[slot] == Some(m) {
                return true;
            }
        }

        false
    }
}

fn score_noisy(board: &Board, moves: &mut MoveList) {
    for scored in moves.iter_mut() {
        scored.score = score_move(board, scored.m);
    }
}

fn score_quiets(board: &Board, history: &ButterflyHistory, moves: &mut MoveList) {
    for scored in moves.iter_mut() {
        scored.score = history[board.us()][scored.m.from()][scored.m.to()];
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

    fn next_move(picker: &mut MovePicker, board: &Board, movegen: &MoveGenerator) -> Option<Move> {
        let history = [[[0; 64]; 64]; 2];

        picker.next_move(board, movegen, &history)
    }

    #[test]
    fn tt_move_is_returned_first() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E4, Square::E5, MoveType::Capture);
        let mut picker = MovePicker::new(Some(tt_move), [None, None], true);

        assert_eq!(next_move(&mut picker, &board, &movegen), Some(tt_move));
    }

    #[test]
    fn tt_move_is_not_returned_twice() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E4, Square::E5, MoveType::Capture);
        let mut picker = MovePicker::new(Some(tt_move), [None, None], true);
        let mut tt_count = 0;

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
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
        let mut picker = MovePicker::new(None, [None, None], true);
        let mut seen_quiet = false;

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            if m.is_capture() || promotion_score(m) > 0 {
                assert!(!seen_quiet);
            } else {
                seen_quiet = true;
            }
        }

        assert!(seen_quiet);
    }

    #[test]
    fn tt_move_is_returned_before_same_killer_once() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let mut picker = MovePicker::new(Some(tt_move), [Some(tt_move), None], true);
        let mut tt_count = 0;

        assert_eq!(next_move(&mut picker, &board, &movegen), Some(tt_move));

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            if m == tt_move {
                tt_count += 1;
            }
        }

        assert_eq!(tt_count, 0);
    }

    #[test]
    fn captures_are_returned_before_killers() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let killer = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let mut picker = MovePicker::new(None, [Some(killer), None], true);
        let first = next_move(&mut picker, &board, &movegen).unwrap();

        assert!(first.is_capture() || promotion_score(first) > 0);

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            if m == killer {
                return;
            }

            assert!(m.is_capture() || promotion_score(m) > 0);
        }

        panic!("killer move was not returned");
    }

    #[test]
    fn killer_quiets_are_returned_before_other_quiets() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let killer = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let mut picker = MovePicker::new(None, [Some(killer), None], true);

        assert_eq!(next_move(&mut picker, &board, &movegen), Some(killer));
    }

    #[test]
    fn quiets_are_ordered_by_history_after_killers() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let killer = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let history_move = Move::new(Square::E1, Square::F1, MoveType::Quiet);
        let mut history = [[[0; 64]; 64]; 2];
        let mut picker = MovePicker::new(None, [Some(killer), None], true);

        history[Color::White][Square::E1][Square::F1] = 100;

        assert_eq!(
            picker.next_move(&board, &movegen, &history),
            Some(killer)
        );
        assert_eq!(
            picker.next_move(&board, &movegen, &history),
            Some(history_move)
        );
    }

    #[test]
    fn duplicate_killers_are_returned_once() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let killer = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let mut picker = MovePicker::new(None, [Some(killer), Some(killer)], true);
        let mut killer_count = 0;

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            if m == killer {
                killer_count += 1;
            }
        }

        assert_eq!(killer_count, 1);
    }

    #[test]
    fn killers_are_skipped_with_quiets() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let killer = Move::new(Square::E1, Square::D1, MoveType::Quiet);
        let mut picker = MovePicker::new(None, [Some(killer), None], false);

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            assert!(m.is_capture() || promotion_score(m) > 0);
            assert_ne!(m, killer);
        }
    }

    #[test]
    fn main_mode_returns_all_noisy_and_quiet_moves() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let noisy = movegen.gen_moves::<Noisy>(&board);
        let quiet = movegen.gen_moves::<Quiet>(&board);
        let mut picker = MovePicker::new(None, [None, None], true);
        let mut count = 0;

        while next_move(&mut picker, &board, &movegen).is_some() {
            count += 1;
        }

        assert_eq!(count, noisy.len() + quiet.len());
    }

    #[test]
    fn quiets_can_be_skipped() {
        let board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let mut picker = MovePicker::new(None, [None, None], false);

        while let Some(m) = next_move(&mut picker, &board, &movegen) {
            assert!(m.is_capture() || promotion_score(m) > 0);
        }
    }

    #[test]
    fn evasion_mode_returns_tt_evasion_first() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4q3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let tt_move = Move::new(Square::E1, Square::E2, MoveType::Capture);
        let mut picker = MovePicker::evasions(Some(tt_move));

        assert_eq!(next_move(&mut picker, &board, &movegen), Some(tt_move));
    }

    #[test]
    fn evasion_mode_returns_generated_evasions() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4q3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let evasions = movegen.gen_moves::<Evasions>(&board);
        let mut picker = MovePicker::evasions(None);
        let mut count = 0;

        while next_move(&mut picker, &board, &movegen).is_some() {
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
