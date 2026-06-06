use arrayvec::ArrayVec;

use crate::movegen::moves::Move;

// https://www.chessprogramming.org/Encoding_Moves#Move_Index
pub(crate) const MAX_MOVES: usize = 256;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct ScoredMove {
    pub m: Move,
    pub score: i32,
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

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut ScoredMove> {
        self.0.iter_mut()
    }

    pub fn pick_next(&mut self, start_idx: usize) -> Move {
        self.pick_next_scored(start_idx).0
    }

    pub fn pick_next_scored(&mut self, start_idx: usize) -> (Move, i32) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::Square;

    #[test]
    fn pick_next_selects_highest_scored_remaining_move() {
        let quiet = Move::new(Square::E1, Square::D1, crate::movegen::moves::MoveType::Quiet);
        let best = Move::new(Square::D4, Square::E5, crate::movegen::moves::MoveType::Capture);
        let middle = Move::new(Square::B4, Square::C5, crate::movegen::moves::MoveType::Capture);
        let mut moves = MoveList::default();

        moves.push(quiet);
        moves.push(middle);
        moves.push(best);
        moves.0[0].score = 0;
        moves.0[1].score = 10;
        moves.0[2].score = 20;

        assert_eq!(moves.pick_next(0), best);
        assert_eq!(moves.pick_next(1), middle);
        assert_eq!(moves.pick_next(2), quiet);
    }

    #[test]
    fn pick_next_scored_returns_selected_move_score() {
        let quiet = Move::new(Square::E1, Square::D1, crate::movegen::moves::MoveType::Quiet);
        let best = Move::new(Square::D4, Square::E5, crate::movegen::moves::MoveType::Capture);
        let mut moves = MoveList::default();

        moves.push(quiet);
        moves.push(best);
        moves.0[0].score = 0;
        moves.0[1].score = 20;

        let (m, score) = moves.pick_next_scored(0);

        assert_eq!(m, best);
        assert_eq!(score, 20);
    }
}
