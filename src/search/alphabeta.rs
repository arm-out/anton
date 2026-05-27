use crate::{
    board::{Board, piece::PieceType, square::Square},
    evaluation::Score,
};

use super::{Search, SearchContext, SearchObserver, SearchResult};

const INF: Score = Score::MAX;
const MATE_SCORE: Score = 30_000;
const DRAW_SCORE: Score = 0;

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
                score: board
                    .state
                    .evaluation
                    .score(board.us(), board.state.game_phase),
            };
        }

        let mut best_move = None;
        let mut best_score = -INF;
        let mut alpha = -INF;
        let beta = INF;
        let mut moves = self.movegen.gen_moves(board);
        moves.score_moves(board);
        let mut legal_moves = 0;

        for i in 0..moves.len() {
            if best_move.is_some() && context.should_stop() {
                break;
            }

            let m = moves.pick_next(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -self.negamax(board, depth - 1, -beta, -alpha, context);
            board.unmake();

            if best_move.is_none() || score > best_score {
                best_score = score;
                best_move = Some(m);
            }

            alpha = alpha.max(score);
        }

        if legal_moves == 0 {
            best_score = self.terminal_score(board);
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

        if board.is_draw() {
            context.leaf();
            return DRAW_SCORE;
        }

        if depth == 0 {
            context.leaf();
            return board
                .state
                .evaluation
                .score(board.us(), board.state.game_phase);
        }

        if context.should_stop() {
            return board
                .state
                .evaluation
                .score(board.us(), board.state.game_phase);
        }

        let mut best_score = -INF;
        let mut moves = self.movegen.gen_moves(board);
        moves.score_moves(board);
        let mut legal_moves = 0;

        for i in 0..moves.len() {
            if context.should_stop() {
                break;
            }

            let m = moves.pick_next(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -self.negamax(board, depth - 1, -beta, -alpha, context);
            board.unmake();

            best_score = best_score.max(score);
            alpha = alpha.max(score);

            if alpha >= beta {
                context.beta_cutoff();
                break;
            }
        }

        if legal_moves == 0 {
            context.leaf();
            return self.terminal_score(board);
        }

        best_score
    }

    fn terminal_score(&self, board: &Board) -> Score {
        let king = board.bitboards[board.us()][PieceType::King];
        let king_square = Square::from_idx(king.0.trailing_zeros() as usize);

        if self.movegen.is_attacked(board, king_square, board.them()) {
            -MATE_SCORE
        } else {
            DRAW_SCORE
        }
    }
}
