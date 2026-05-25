use crate::{
    board::Board,
    evaluation::Score,
    movegen::{MoveGenerator, moves::Move},
};

const SEARCH_DEPTH: u8 = 2;
const INF: Score = Score::MAX;

#[derive(Debug, Default, PartialEq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: Score,
}

pub struct Search {
    movegen: MoveGenerator,
}

impl Search {
    pub fn new() -> Self {
        Self {
            movegen: MoveGenerator::new(),
        }
    }

    pub fn search(&self, board: &mut Board) -> SearchResult {
        let mut best_move = None;
        let mut best_score = -INF;
        let mut alpha = -INF;
        let beta = INF;
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, SEARCH_DEPTH - 1, -beta, -alpha);
            board.unmake();

            if score > best_score {
                best_score = score;
                best_move = Some(m);
            }

            alpha = alpha.max(score);
        }

        SearchResult {
            best_move,
            score: best_score,
        }
    }

    fn negamax(&self, board: &mut Board, depth: u8, mut alpha: Score, beta: Score) -> Score {
        if depth == 0 {
            return board.state.evaluation.score(board.us());
        }

        let mut best_score = -INF;
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, depth - 1, -beta, -alpha);
            board.unmake();

            best_score = best_score.max(score);
            alpha = alpha.max(score);

            if alpha >= beta {
                break;
            }
        }

        best_score
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::Square;

    #[test]
    fn searches_two_plies_and_returns_a_move() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search(&mut board);

        assert!(result.best_move.is_some());
        assert_eq!(board.us(), crate::board::piece::Color::White);
        assert_eq!(board.history.len(), 0);
    }

    #[test]
    fn chooses_free_material_at_depth_two() {
        let mut board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search(&mut board);

        let best_move = result.best_move.unwrap();
        assert_eq!(best_move.from(), Square::E4);
        assert_eq!(best_move.to(), Square::E5);
    }
}
