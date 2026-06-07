use arrayvec::ArrayVec;

use crate::{
    board::{Board, piece::PieceType, square::Square},
    evaluation::{Score, evaluate_static},
    movegen::{
        MAX_MOVES, MoveGenerator,
        moves::{Move, MoveType},
    },
};

use super::{
    Search, SearchInfo, SearchRefs, SearchResult, is_mate_score,
    movepicker::MovePicker,
    transposition::{Bound, TTEntry},
};

pub(super) const INF: Score = Score::MAX;
pub(super) const MATE_SCORE: Score = 30_000;
pub(super) const ASPIRATION_WINDOW: Score = 50;
pub(super) const ASPIRATION_MAX_WINDOW: Score = INF;
const DRAW_SCORE: Score = 0;
const ROOT_PLY: u8 = 0;
const REVERSE_FUTILITY_MAX_DEPTH: u8 = 3;
const REVERSE_FUTILITY_MARGIN: Score = 80;
const NULL_MOVE_MIN_DEPTH: u8 = 3;
const LMR_MIN_DEPTH: u8 = 3;
const LMR_MIN_MOVES: u32 = 4;

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
        let in_check = Self::in_check(refs.board, refs.movegen);
        let mut move_picker = if in_check {
            MovePicker::evasions(tt_move)
        } else {
            MovePicker::new(tt_move, [None, None], true)
        };
        let mut legal_moves = 0;

        while let Some(m) = move_picker.next_move(refs.board, refs.movegen, refs.history) {
            if best_move.is_some() && info.should_stop() {
                break;
            }

            if !refs.board.make(m, refs.movegen) {
                continue;
            }

            legal_moves += 1;
            let score = -Self::negamax::<true>(
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

    fn negamax<const PV: bool>(
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
            return evaluate_static(refs.board, refs.movegen);
        }

        let key = refs.board.state.zobrist_key;
        let original_alpha = alpha;
        let tt_entry = refs.tt.probe(key);

        if let Some(entry) = tt_entry
            && let Some(score) = tt_cutoff(entry, depth, alpha, beta, ply)
        {
            return score;
        }

        let static_eval = evaluate_static(refs.board, refs.movegen);
        let in_check = Self::in_check(refs.board, refs.movegen);

        // Reverse futility pruning
        if depth <= REVERSE_FUTILITY_MAX_DEPTH
            && !PV
            && !in_check
            && !is_mate_score(beta)
            && static_eval.saturating_sub(REVERSE_FUTILITY_MARGIN * depth as Score) >= beta
        {
            return static_eval;
        }

        // Null move pruning
        if !PV
            && depth >= NULL_MOVE_MIN_DEPTH
            && !in_check
            && static_eval >= beta
            && !is_mate_score(beta)
            && can_null_move(refs.board)
            && !info.should_stop()
        {
            let reduction = 2 + depth / 4;
            refs.board.make_null();
            let score = -Self::negamax::<false>(
                refs,
                depth - 1 - reduction,
                -beta,
                (-beta).saturating_add(1),
                ply + 1,
                info,
            );
            refs.board.unmake_null();

            if score >= beta {
                return beta;
            }
        }

        let tt_move = tt_entry.map(|entry| entry.best_move());
        let killers = Self::killer_moves(refs.killers, ply);
        let mut best_move = None;
        let mut best_score = -INF;
        let mut move_picker = if in_check {
            MovePicker::evasions(tt_move)
        } else {
            MovePicker::new(tt_move, killers, true)
        };
        let mut legal_moves = 0;

        let mut searched_quiets: ArrayVec<Move, MAX_MOVES> = ArrayVec::new();

        while let Some(m) = move_picker.next_move(refs.board, refs.movegen, refs.history) {
            if info.should_stop() {
                break;
            }

            let color = refs.board.us();
            let quiet = is_quiet_history_move(m);

            if !refs.board.make(m, refs.movegen) {
                continue;
            }

            legal_moves += 1;
            let gives_check = Self::in_check(refs.board, refs.movegen);

            // PVS search
            let mut score = if PV && legal_moves == 1 {
                -Self::negamax::<true>(refs, depth - 1, -beta, -alpha, ply + 1, info)
            } else {
                let null_beta = alpha.saturating_add(1);
                // Late move reduction
                let reduction = if depth >= LMR_MIN_DEPTH
                    && legal_moves >= LMR_MIN_MOVES
                    && quiet
                    && !in_check
                    && !gives_check
                    && !is_mate_score(alpha)
                    && !is_mate_score(beta)
                    && !info.stopped
                {
                    lmr_reduction(depth, legal_moves)
                } else {
                    0
                };

                let score = if reduction > 0 {
                    -Self::negamax::<false>(
                        refs,
                        depth - 1 - reduction,
                        -null_beta,
                        -alpha,
                        ply + 1,
                        info,
                    )
                } else {
                    -INF
                };

                if reduction == 0 || score > alpha {
                    -Self::negamax::<false>(refs, depth - 1, -null_beta, -alpha, ply + 1, info)
                } else {
                    score
                }
            };

            // Null window fail -> search with full window
            if legal_moves > 1 && score > alpha && score < beta {
                score = -Self::negamax::<true>(refs, depth - 1, -beta, -alpha, ply + 1, info);
            }
            refs.board.unmake();

            if best_move.is_none() || score > best_score {
                best_score = score;
                best_move = Some(m);
            }
            alpha = alpha.max(score);

            if alpha >= beta {
                info.beta_cutoff();
                if !info.stopped && quiet {
                    Self::update_history(refs.history, color, m, depth, &searched_quiets);
                    Self::update_killer(refs.killers, ply, m);
                }
                break;
            }

            if quiet {
                searched_quiets.push(m);
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
        let stand_pat = evaluate_static(refs.board, refs.movegen);

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

        let mut move_picker = if in_check {
            MovePicker::evasions(None)
        } else {
            MovePicker::new(None, [None, None], false)
        };
        let mut legal_moves = 0;

        while let Some(m) = move_picker.next_move(refs.board, refs.movegen, refs.history) {
            if info.should_stop() {
                info.leaf();
                return if legal_moves == 0 {
                    stand_pat
                } else {
                    best_score
                };
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

fn is_quiet_history_move(m: Move) -> bool {
    matches!(
        m.kind(),
        MoveType::Quiet
            | MoveType::DoublePawnPush
            | MoveType::CastleKingside
            | MoveType::CastleQueenside
    )
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

fn can_null_move(board: &Board) -> bool {
    let pieces = board.bitboards[board.us()];

    pieces[PieceType::Knight].0 != 0
        || pieces[PieceType::Bishop].0 != 0
        || pieces[PieceType::Rook].0 != 0
        || pieces[PieceType::Queen].0 != 0
}

fn lmr_reduction(depth: u8, legal_moves: u32) -> u8 {
    if depth < LMR_MIN_DEPTH || legal_moves < LMR_MIN_MOVES {
        return 0;
    }

    let reduction = 1.0 + (depth as f64).ln() * (legal_moves as f64).ln() / 3.14;
    let reduction = reduction as u8;
    let max_reduction = depth.saturating_sub(2);

    reduction.min(max_reduction)
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
    fn null_move_requires_non_pawn_material() {
        let king_only = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let pawn_only = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let knight = Board::from_fen("4k3/8/8/8/8/8/8/4K1N1 w - - 0 1").unwrap();
        let bishop = Board::from_fen("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        let rook = Board::from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap();
        let queen = Board::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();

        assert!(!can_null_move(&king_only));
        assert!(!can_null_move(&pawn_only));
        assert!(can_null_move(&knight));
        assert!(can_null_move(&bishop));
        assert!(can_null_move(&rook));
        assert!(can_null_move(&queen));
    }

    #[test]
    fn lmr_reduction_obeys_minimums() {
        assert_eq!(lmr_reduction(2, 4), 0);
        assert_eq!(lmr_reduction(3, 3), 0);
    }

    #[test]
    fn lmr_reduction_uses_log_formula() {
        let expected = (1.0 + (3.0_f64).ln() * (4.0_f64).ln() / 3.14) as u8;

        assert_eq!(lmr_reduction(3, 4), expected);
    }

    #[test]
    fn lmr_reduction_grows_and_keeps_child_depth() {
        let shallow = lmr_reduction(4, 4);
        let deeper = lmr_reduction(8, 12);

        assert!(deeper >= shallow);
        assert!(lmr_reduction(3, 64) <= 1);
        assert!(lmr_reduction(8, 64) <= 6);
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
                killers: &mut search.killers,
                history: &mut search.history,
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
                killers: &mut search.killers,
                history: &mut search.history,
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
