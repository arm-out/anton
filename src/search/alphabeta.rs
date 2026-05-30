use crate::{
    board::{Board, piece::PieceType, square::Square},
    evaluation::Score,
    movegen::MoveGenerator,
};

use super::{
    Search, SearchInfo, SearchRefs, SearchResult, is_mate_score,
    transposition::{Bound, TTEntry},
};

pub(super) const INF: Score = Score::MAX;
pub(super) const MATE_SCORE: Score = 30_000;
pub(super) const ASPIRATION_WINDOW: Score = 50;
pub(super) const ASPIRATION_MAX_WINDOW: Score = INF;
const DRAW_SCORE: Score = 0;
const QUIET_MOVE_SCORE: i16 = 0;
const ROOT_PLY: u8 = 0;
const REVERSE_FUTILITY_MARGIN: Score = 80;

impl Search {
    pub(super) fn search_depth_inner(
        mut refs: SearchRefs<'_>,
        depth: u8,
        alpha: Score,
        beta: Score,
        info: &mut SearchInfo,
    ) -> SearchResult {
        info.root();

        if depth == 0 {
            info.completed_depth = true;

            return SearchResult {
                best_move: None,
                score: Self::quiescence(&mut refs, -INF, INF, ROOT_PLY, info),
                depth,
                stats: info.stats,
            };
        }

        let key = refs.board.state.zobrist_key;
        let tt_move = refs.tt.probe(key).map(|entry| entry.best_move());
        let mut best_move = None;
        let mut best_score = -INF;
        let original_alpha = alpha;
        let mut alpha = alpha;
        let mut moves = refs.movegen.gen_moves(refs.board);
        moves.score_moves(refs.board, tt_move);
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
            let score = -Self::negamax(
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

            if alpha >= beta {
                info.beta_cutoff();
                break;
            }
        }

        if legal_moves == 0 {
            best_score = Self::terminal_score(refs.board, refs.movegen, ROOT_PLY);
        } else if info.completed_depth || !info.stopped {
            let bound = if best_score <= original_alpha {
                Bound::Upper
            } else if best_score >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };

            refs.tt.store(
                key,
                best_move.unwrap(),
                score_to_tt(best_score, ROOT_PLY),
                depth,
                bound,
            );
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
            return Self::quiescence(refs, alpha, beta, ply, info);
        }

        if info.should_stop() {
            return refs
                .board
                .state
                .evaluation
                .score(refs.board.us(), refs.board.state.game_phase);
        }

        let key = refs.board.state.zobrist_key;
        let original_alpha = alpha;
        let tt_entry = refs.tt.probe(key);

        if let Some(entry) = tt_entry
            && let Some(score) = tt_cutoff(entry, depth, alpha, beta, ply)
        {
            return score;
        }

        let static_eval = refs
            .board
            .state
            .evaluation
            .score(refs.board.us(), refs.board.state.game_phase);
        let in_check = Self::in_check(refs.board, refs.movegen);

        if !in_check
            && !is_mate_score(beta)
            && static_eval.saturating_sub(REVERSE_FUTILITY_MARGIN * depth as Score) >= beta
        {
            return static_eval;
        }

        let tt_move = tt_entry.map(|entry| entry.best_move());
        let mut best_move = None;
        let mut best_score = -INF;
        let mut moves = refs.movegen.gen_moves(refs.board);
        moves.score_moves(refs.board, tt_move);
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

            // PVS search
            let mut score = if legal_moves == 1 {
                -Self::negamax(refs, depth - 1, -beta, -alpha, ply + 1, info)
            } else {
                let null_beta = alpha.saturating_add(1);
                -Self::negamax(refs, depth - 1, -null_beta, -alpha, ply + 1, info)
            };

            // Null window fail -> search with full window
            if legal_moves > 1 && score > alpha && score < beta {
                score = -Self::negamax(refs, depth - 1, -beta, -alpha, ply + 1, info);
            }
            refs.board.unmake();

            if best_move.is_none() || score > best_score {
                best_score = score;
                best_move = Some(m);
            }
            alpha = alpha.max(score);

            if alpha >= beta {
                info.beta_cutoff();
                break;
            }
        }

        if legal_moves == 0 {
            info.leaf();
            return Self::terminal_score(refs.board, refs.movegen, ply);
        }

        if !info.stopped {
            let bound = if best_score <= original_alpha {
                Bound::Upper
            } else if best_score >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };

            refs.tt.store(
                key,
                best_move.unwrap(),
                score_to_tt(best_score, ply),
                depth,
                bound,
            );
        }

        best_score
    }

    fn quiescence(
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

        let in_check = Self::in_check(refs.board, refs.movegen);
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
        moves.score_moves(refs.board, None);
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
            let score = -Self::quiescence(refs, -beta, -alpha, ply + 1, info);
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
                return Self::mated_score(ply);
            }
        }

        best_score
    }

    fn terminal_score(board: &Board, movegen: &MoveGenerator, ply: u8) -> Score {
        if Self::in_check(board, movegen) {
            Self::mated_score(ply)
        } else {
            DRAW_SCORE
        }
    }

    fn mated_score(ply: u8) -> Score {
        -(MATE_SCORE - ply as Score)
    }

    fn in_check(board: &Board, movegen: &MoveGenerator) -> bool {
        let king = board.bitboards[board.us()][PieceType::King];
        let king_square = Square::from_idx(king.0.trailing_zeros() as usize);

        movegen.is_attacked(board, king_square, board.them())
    }
}

fn tt_cutoff(entry: TTEntry, depth: u8, alpha: Score, beta: Score, ply: u8) -> Option<Score> {
    if entry.depth() < depth {
        return None;
    }

    let score = score_from_tt(entry.score(), ply);

    match entry.bound() {
        Bound::Exact => Some(score),
        Bound::Lower if score >= beta => Some(score),
        Bound::Upper if score <= alpha => Some(score),
        _ => None,
    }
}

fn score_to_tt(score: Score, ply: u8) -> Score {
    if score > MATE_SCORE - 256 {
        score + ply as Score
    } else if score < -MATE_SCORE + 256 {
        score - ply as Score
    } else {
        score
    }
}

fn score_from_tt(score: Score, ply: u8) -> Score {
    if score > MATE_SCORE - 256 {
        score - ply as Score
    } else if score < -MATE_SCORE + 256 {
        score + ply as Score
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{DEFAULT_TT_SIZE_MB, SearchLimit};
    use crate::{
        board::square::Square,
        movegen::moves::{Move, MoveType},
    };

    fn entry(score: Score, depth: u8, bound: Bound) -> TTEntry {
        TTEntry::new(
            1,
            Move::new(Square::E2, Square::E4, MoveType::Quiet),
            score_to_tt(score, 2),
            depth,
            bound,
        )
    }

    #[test]
    fn exact_tt_bound_returns_score() {
        assert_eq!(
            tt_cutoff(entry(42, 4, Bound::Exact), 4, -100, 100, 2),
            Some(42)
        );
    }

    #[test]
    fn lower_tt_bound_only_cuts_at_or_above_beta() {
        assert_eq!(
            tt_cutoff(entry(100, 4, Bound::Lower), 4, -100, 50, 2),
            Some(100)
        );
        assert_eq!(tt_cutoff(entry(25, 4, Bound::Lower), 4, -100, 50, 2), None);
    }

    #[test]
    fn upper_tt_bound_only_cuts_at_or_below_alpha() {
        assert_eq!(
            tt_cutoff(entry(-100, 4, Bound::Upper), 4, -50, 50, 2),
            Some(-100)
        );
        assert_eq!(tt_cutoff(entry(25, 4, Bound::Upper), 4, -50, 50, 2), None);
    }

    #[test]
    fn shallow_tt_entry_does_not_cut_off_deeper_search() {
        assert_eq!(tt_cutoff(entry(42, 3, Bound::Exact), 4, -100, 100, 2), None);
    }

    #[test]
    fn mate_scores_are_adjusted_for_current_ply() {
        let stored = score_to_tt(29_997, 3);

        assert_eq!(score_from_tt(stored, 1), 29_999);
        assert_eq!(score_from_tt(stored, 5), 29_995);
    }

    #[test]
    fn root_fail_high_stores_lower_tt_bound() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let key = board.state.zobrist_key;
        let mut search = Search::new(DEFAULT_TT_SIZE_MB);
        let mut info = SearchInfo::new(SearchLimit::Depth(1));

        let result = Search::search_depth_inner(
            SearchRefs {
                board: &mut board,
                movegen: &search.movegen,
                tt: &mut search.tt,
            },
            1,
            -10_001,
            -10_000,
            &mut info,
        );

        assert!(result.score >= -10_000);
        assert_eq!(search.tt.probe(key).unwrap().bound(), Bound::Lower);
    }

    #[test]
    fn root_fail_low_stores_upper_tt_bound() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let key = board.state.zobrist_key;
        let mut search = Search::new(DEFAULT_TT_SIZE_MB);
        let mut info = SearchInfo::new(SearchLimit::Depth(1));

        let result = Search::search_depth_inner(
            SearchRefs {
                board: &mut board,
                movegen: &search.movegen,
                tt: &mut search.tt,
            },
            1,
            10_000,
            10_001,
            &mut info,
        );

        assert!(result.score <= 10_000);
        assert_eq!(search.tt.probe(key).unwrap().bound(), Bound::Upper);
    }
}
