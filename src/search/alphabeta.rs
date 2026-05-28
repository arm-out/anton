use crate::{
    board::{Board, piece::PieceType, square::Square},
    evaluation::Score,
    movegen::MoveGenerator,
};

use super::{Search, SearchInfo, SearchRefs, SearchResult};

const INF: Score = Score::MAX;
const MATE_SCORE: Score = 30_000;
const DRAW_SCORE: Score = 0;

impl Search {
    pub(super) fn search_depth_inner(
        &self,
        mut refs: SearchRefs<'_>,
        depth: u8,
        info: &mut SearchInfo,
    ) -> SearchResult {
        info.root();

        if depth == 0 {
            info.leaf();
            info.completed_depth = true;

            return SearchResult {
                best_move: None,
                score: refs
                    .board
                    .state
                    .evaluation
                    .score(refs.board.us(), refs.board.state.game_phase),
                depth,
                stats: info.stats,
            };
        }

        let mut best_move = None;
        let mut best_score = -INF;
        let mut alpha = -INF;
        let beta = INF;
        let mut moves = refs.movegen.gen_moves(refs.board);
        moves.score_moves(refs.board);
        let mut legal_moves = 0;

        for i in 0..moves.len() {
            if best_move.is_some() && info.should_stop() {
                break;
            }

            let m = moves.pick_next(i);

            if !refs.board.make(m, refs.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -self.negamax(&mut refs, depth - 1, -beta, -alpha, info);
            refs.board.unmake();

            if best_move.is_none() || score > best_score {
                best_score = score;
                best_move = Some(m);
            }

            alpha = alpha.max(score);
        }

        if legal_moves == 0 {
            best_score = self.terminal_score(refs.board, refs.movegen);
        }

        info.completed_depth = !info.stopped;

        SearchResult {
            best_move,
            score: best_score,
            depth,
            stats: info.stats,
        }
    }

    fn negamax(
        &self,
        refs: &mut SearchRefs<'_>,
        depth: u8,
        mut alpha: Score,
        beta: Score,
        info: &mut SearchInfo,
    ) -> Score {
        info.node();

        if refs.board.is_draw() {
            info.leaf();
            return DRAW_SCORE;
        }

        if depth == 0 {
            info.leaf();
            return refs
                .board
                .state
                .evaluation
                .score(refs.board.us(), refs.board.state.game_phase);
        }

        if info.should_stop() {
            return refs
                .board
                .state
                .evaluation
                .score(refs.board.us(), refs.board.state.game_phase);
        }

        let mut best_score = -INF;
        let mut moves = refs.movegen.gen_moves(refs.board);
        moves.score_moves(refs.board);
        let mut legal_moves = 0;

        for i in 0..moves.len() {
            if info.should_stop() {
                break;
            }

            let m = moves.pick_next(i);

            if !refs.board.make(m, refs.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -self.negamax(refs, depth - 1, -beta, -alpha, info);
            refs.board.unmake();

            best_score = best_score.max(score);
            alpha = alpha.max(score);

            if alpha >= beta {
                info.beta_cutoff();
                break;
            }
        }

        if legal_moves == 0 {
            info.leaf();
            return self.terminal_score(refs.board, refs.movegen);
        }

        best_score
    }

    fn terminal_score(&self, board: &Board, movegen: &MoveGenerator) -> Score {
        let king = board.bitboards[board.us()][PieceType::King];
        let king_square = Square::from_idx(king.0.trailing_zeros() as usize);

        if movegen.is_attacked(board, king_square, board.them()) {
            -MATE_SCORE
        } else {
            DRAW_SCORE
        }
    }
}
