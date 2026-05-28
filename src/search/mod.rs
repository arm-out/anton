mod alphabeta;
mod time;

use std::time::{Duration, Instant};

use crate::{
    board::Board,
    evaluation::Score,
    movegen::{MoveGenerator, moves::Move},
};

use self::time::{TIME_CHECK_INTERVAL, deadline_for, max_depth_for};

const DEFAULT_SEARCH_DEPTH: u8 = 5;
const MAX_SEARCH_DEPTH: u8 = u8::MAX;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SearchLimit {
    Depth(u8),
    MoveTime(Duration),
    Clock {
        remaining: Duration,
        increment: Duration,
    },
    Infinite,
}

#[derive(Debug, Default, PartialEq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: Score,
    pub stats: SearchStats,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SearchStats {
    pub nodes: u64,
    pub leaves: u64,
    pub beta_cutoffs: u64,
}

struct SearchRefs<'a> {
    board: &'a mut Board,
    movegen: &'a MoveGenerator,
}

struct SearchInfo {
    limit: SearchLimit,
    stats: SearchStats,
    deadline: Option<Instant>,
    stopped: bool,
    completed_depth: bool,
}

impl SearchInfo {
    fn new(limit: SearchLimit) -> Self {
        Self {
            limit,
            stats: SearchStats::default(),
            deadline: deadline_for(limit),
            stopped: false,
            completed_depth: false,
        }
    }

    fn root(&mut self) {
        self.stats.nodes += 1;
    }

    fn node(&mut self) {
        self.stats.nodes += 1;
    }

    fn leaf(&mut self) {
        self.stats.leaves += 1;
    }

    fn beta_cutoff(&mut self) {
        self.stats.beta_cutoffs += 1;
    }

    fn max_depth(&self) -> u8 {
        max_depth_for(self.limit)
    }

    fn should_stop(&mut self) -> bool {
        if self.stopped {
            return true;
        }

        if self.deadline.is_none() {
            return false;
        }

        if self.stats.nodes > 1 && self.stats.nodes % TIME_CHECK_INTERVAL != 0 {
            return false;
        }

        self.stop_if_expired()
    }

    fn stop_if_expired(&mut self) -> bool {
        if self.stopped {
            return true;
        }

        let Some(deadline) = self.deadline else {
            return false;
        };

        self.stopped = Instant::now() >= deadline;
        self.stopped
    }
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

    pub fn search(&self, board: &mut Board, limit: SearchLimit) -> SearchResult {
        let mut info = SearchInfo::new(limit);
        let max_depth = info.max_depth();
        let mut best_result = None;

        for depth in 1..=max_depth {
            if depth > 1 && info.stop_if_expired() {
                break;
            }

            info.completed_depth = false;
            let mut result = self.search_depth_inner(
                SearchRefs {
                    board,
                    movegen: &self.movegen,
                },
                depth,
                &mut info,
            );
            let completed = info.completed_depth;

            if completed || best_result.is_none() {
                result.stats = info.stats;
                best_result = Some(result);
            }
        }

        let mut result = best_result.unwrap_or_default();
        result.stats = info.stats;
        result
    }

    pub fn search_depth(&self, board: &mut Board, depth: u8) -> SearchResult {
        let mut info = SearchInfo::new(SearchLimit::Depth(depth));
        self.search_depth_inner(
            SearchRefs {
                board,
                movegen: &self.movegen,
            },
            depth,
            &mut info,
        )
    }

    pub fn apply_uci_move(&self, board: &mut Board, uci_move: &str) -> Result<(), String> {
        let Some(m) = self.find_legal_move(board, uci_move) else {
            return Err(format!("invalid move: {uci_move}"));
        };

        if !board.make(m, &self.movegen) {
            return Err(format!("illegal move: {uci_move}"));
        }

        Ok(())
    }

    fn find_legal_move(&self, board: &Board, uci_move: &str) -> Option<Move> {
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if m.to_uci() == uci_move {
                return Some(m);
            }
        }

        None
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_limit() -> SearchLimit {
    SearchLimit::Depth(DEFAULT_SEARCH_DEPTH)
}

fn infinite_depth() -> u8 {
    MAX_SEARCH_DEPTH
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::Square;

    #[test]
    fn searches_two_plies_and_returns_a_move() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search(&mut board, default_limit());

        assert!(result.best_move.is_some());
        assert_eq!(board.us(), crate::board::piece::Color::White);
        assert_eq!(board.history.len(), 0);
    }

    #[test]
    fn chooses_free_material_at_depth_two() {
        let mut board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search(&mut board, default_limit());

        let best_move = result.best_move.unwrap();
        assert_eq!(best_move.from(), Square::E4);
        assert_eq!(best_move.to(), Square::E5);
    }

    #[test]
    fn finds_move_in_exposed_black_king_position() {
        let mut board =
            Board::from_fen("kr3b1r/p5pp/p1Qp4/3P4/1P6/P1R5/2P2PPP/2K1R3 b - - 2 23").unwrap();
        let search = Search::new();

        let result = search.search(&mut board, default_limit());

        assert_eq!(result.best_move.unwrap().to_uci(), "b8b7");
    }

    #[test]
    fn finds_move_when_only_terrible_moves_are_available() {
        let mut board = Board::from_fen("2R5/7p/1p2pQp1/8/5k2/P7/1P3PPP/4K2R b K - 2 41").unwrap();
        let search = Search::new();

        let result = search.search(&mut board, default_limit());

        assert!(result.best_move.is_some());
    }

    #[test]
    fn iterative_depth_returns_fixed_depth_result() {
        let mut iterative_board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let mut fixed_board = iterative_board.clone();
        let search = Search::new();

        let iterative = search.search(&mut iterative_board, SearchLimit::Depth(3));
        let fixed = search.search_depth(&mut fixed_board, 3);

        assert_eq!(iterative.best_move, fixed.best_move);
        assert_eq!(iterative.score, fixed.score);
        assert_eq!(iterative_board.history.len(), 0);
    }

    #[test]
    fn search_results_always_include_stats() {
        let mut default_board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let mut iterative_board = default_board.clone();
        let mut fixed_board = default_board.clone();
        let search = Search::new();

        let default = search.search(&mut default_board, default_limit());
        let iterative = search.search(&mut iterative_board, SearchLimit::Depth(1));
        let fixed = search.search_depth(&mut fixed_board, 1);

        assert!(default.stats.nodes > 0);
        assert!(iterative.stats.nodes > 0);
        assert!(fixed.stats.nodes > 0);
    }

    #[test]
    fn iterative_depth_one_and_two_return_legal_moves() {
        let search = Search::new();

        for depth in [1, 2] {
            let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
            let result = search.search(&mut board, SearchLimit::Depth(depth));

            assert!(result.best_move.is_some());
            assert_eq!(board.history.len(), 0);
        }
    }

    #[test]
    fn tiny_movetime_returns_without_panicking() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search(&mut board, SearchLimit::MoveTime(Duration::from_millis(1)));

        assert_eq!(board.history.len(), 0);
        assert!(result.best_move.is_some() || result == SearchResult::default());
    }

    #[test]
    fn repeated_root_position_still_returns_a_move() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/8/4K1N1 w - - 0 1").unwrap();
        let search = Search::new();

        for uci_move in ["g1f3", "e8d8", "f3g1", "d8e8"] {
            search.apply_uci_move(&mut board, uci_move).unwrap();
        }

        let result = search.search(&mut board, default_limit());

        assert!(result.best_move.is_some());
    }

    #[test]
    fn returns_move_in_book_repetition_position() {
        let mut board =
            Board::from_fen("r1bqk1nr/pp1pppbp/2n3p1/2p5/4P3/N5PN/PPPP1PBP/R1BQ1RK1 w - - 18 13")
                .unwrap();
        let search = Search::new();

        let result = search.search_depth(&mut board, 1);

        assert!(result.best_move.is_some());
    }

    #[test]
    fn checkmate_position_scores_as_loss_for_side_to_move() {
        let mut board = Board::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search_depth(&mut board, 1);

        assert!(result.best_move.is_none());
        assert!(result.score < -20_000);
    }

    #[test]
    fn stalemate_position_scores_as_draw() {
        let mut board = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search_depth(&mut board, 1);

        assert!(result.best_move.is_none());
        assert_eq!(result.score, 0);
    }
}
