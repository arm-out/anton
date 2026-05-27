use crate::evaluation::Score;

use super::{Search, SearchContext, SearchObserver, SearchResult};

const INF: Score = Score::MAX;

impl Search {
    pub(super) fn search_depth_inner<O: SearchObserver>(
        &self,
        board: &mut crate::board::Board,
        depth: u8,
        context: &mut SearchContext<O>,
    ) -> SearchResult {
        context.root();

        if depth == 0 {
            context.leaf();

            return SearchResult {
                best_move: None,
                score: board.state.evaluation.score(board.us()),
            };
        }

        let mut best_move = None;
        let mut best_score = -INF;
        let mut alpha = -INF;
        let beta = INF;
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            if context.should_stop() {
                break;
            }

            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, depth - 1, -beta, -alpha, context);
            board.unmake();

            if best_move.is_none() || score > best_score {
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

    fn negamax<O: SearchObserver>(
        &self,
        board: &mut crate::board::Board,
        depth: u8,
        mut alpha: Score,
        beta: Score,
        context: &mut SearchContext<O>,
    ) -> Score {
        context.node();

        if depth == 0 {
            context.leaf();
            return board.state.evaluation.score(board.us());
        }

        if context.should_stop() {
            return board.state.evaluation.score(board.us());
        }

        let mut best_score = -INF;
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            if context.should_stop() {
                break;
            }

            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, depth - 1, -beta, -alpha, context);
            board.unmake();

            best_score = best_score.max(score);
            alpha = alpha.max(score);

            if alpha >= beta {
                context.beta_cutoff();
                break;
            }
        }

        best_score
    }
}
