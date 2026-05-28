use crate::{
    board::{Board, piece::PieceType, square::Square},
    evaluation::Score,
    movegen::MoveGenerator,
};

use super::{Search, SearchInfo, SearchRefs, SearchResult};

const INF: Score = Score::MAX;
const MATE_SCORE: Score = 30_000;
const DRAW_SCORE: Score = 0;
const QUIET_MOVE_SCORE: i16 = 0;
const ROOT_PLY: u8 = 0;

impl Search {
    pub(super) fn search_depth_inner(
        &self,
        mut refs: SearchRefs<'_>,
        depth: u8,
        info: &mut SearchInfo,
    ) -> SearchResult {
        info.root();

        if depth == 0 {
            info.completed_depth = true;

            return SearchResult {
                best_move: None,
                score: self.quiescence(&mut refs, -INF, INF, ROOT_PLY, info),
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
            let score = -self.negamax(
                &mut refs,
                depth - 1,
                -beta,
                -alpha,
                ROOT_PLY.saturating_add(1),
                info,
            );
            refs.board.unmake();

            if best_move.is_none() || score > best_score {
                best_score = score;
                best_move = Some(m);
            }

            alpha = alpha.max(score);
        }

        if legal_moves == 0 {
            best_score = self.terminal_score(refs.board, refs.movegen, ROOT_PLY);
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
        ply: u8,
        info: &mut SearchInfo,
    ) -> Score {
        info.node();

        if refs.board.is_draw() {
            info.leaf();
            return DRAW_SCORE;
        }

        if depth == 0 {
            return self.quiescence(refs, alpha, beta, ply, info);
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
            let score = -self.negamax(refs, depth - 1, -beta, -alpha, ply.saturating_add(1), info);
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
            return self.terminal_score(refs.board, refs.movegen, ply);
        }

        best_score
    }

    fn quiescence(
        &self,
        refs: &mut SearchRefs<'_>,
        mut alpha: Score,
        beta: Score,
        ply: u8,
        info: &mut SearchInfo,
    ) -> Score {
        info.qnode();

        if refs.board.is_draw() {
            info.leaf();
            return DRAW_SCORE;
        }

        let in_check = self.in_check(refs.board, refs.movegen);
        let stand_pat = refs
            .board
            .state
            .evaluation
            .score(refs.board.us(), refs.board.state.game_phase);

        if info.should_stop() {
            info.leaf();
            return stand_pat;
        }

        let mut best_score = -INF;

        if !in_check {
            if stand_pat >= beta {
                info.beta_cutoff();
                return beta;
            }

            alpha = alpha.max(stand_pat);
            best_score = alpha;
        }

        let mut moves = refs.movegen.gen_moves(refs.board);
        moves.score_moves(refs.board);
        let mut legal_moves = 0;

        for i in 0..moves.len() {
            if info.should_stop() {
                info.leaf();
                return if legal_moves == 0 {
                    stand_pat
                } else {
                    best_score
                };
            }

            let (m, score) = moves.pick_next_scored(i);
            // We want to check all legal evasions if in check
            if !in_check && score <= QUIET_MOVE_SCORE {
                break;
            }

            if !refs.board.make(m, refs.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -self.quiescence(refs, -beta, -alpha, ply + 1, info);
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

            if in_check {
                return self.mated_score(ply);
            }
        }

        best_score
    }

    fn terminal_score(&self, board: &Board, movegen: &MoveGenerator, ply: u8) -> Score {
        if self.in_check(board, movegen) {
            self.mated_score(ply)
        } else {
            DRAW_SCORE
        }
    }

    fn mated_score(&self, ply: u8) -> Score {
        -(MATE_SCORE - ply as Score)
    }

    fn in_check(&self, board: &Board, movegen: &MoveGenerator) -> bool {
        let king = board.bitboards[board.us()][PieceType::King];
        let king_square = Square::from_idx(king.0.trailing_zeros() as usize);

        movegen.is_attacked(board, king_square, board.them())
    }
}
