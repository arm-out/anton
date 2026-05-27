mod alphabeta;
mod time;

use std::time::Duration;

use crate::{
    board::Board,
    evaluation::Score,
    movegen::{MoveGenerator, moves::Move},
};

use self::time::TimeManager;

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
}

#[cfg(feature = "search-stats")]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SearchStats {
    pub nodes: u64,
    pub leaves: u64,
    pub beta_cutoffs: u64,
}

trait SearchObserver {
    fn root(&mut self) {}
    fn node(&mut self) {}
    fn leaf(&mut self) {}
    fn beta_cutoff(&mut self) {}
}

struct NoStats;

impl SearchObserver for NoStats {}

#[cfg(feature = "search-stats")]
impl SearchObserver for SearchStats {
    fn root(&mut self) {
        self.nodes += 1;
    }

    fn node(&mut self) {
        self.nodes += 1;
    }

    fn leaf(&mut self) {
        self.leaves += 1;
    }

    fn beta_cutoff(&mut self) {
        self.beta_cutoffs += 1;
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
        let timer = TimeManager::new(limit);
        let max_depth = timer.max_depth();
        let mut best_result = None;

        for depth in 1..=max_depth {
            if timer.should_stop() {
                break;
            }

            let (result, completed) = self.search_depth_timed(board, depth, &timer);

            if completed || best_result.is_none() {
                best_result = Some(result);
            }

            if timer.should_stop() {
                break;
            }
        }

        best_result.unwrap_or_default()
    }

    pub fn search_default(&self, board: &mut Board) -> SearchResult {
        self.search(board, SearchLimit::Depth(DEFAULT_SEARCH_DEPTH))
    }

    pub fn search_depth(&self, board: &mut Board, depth: u8) -> SearchResult {
        let mut observer = NoStats;

        self.search_depth_inner(board, depth, &mut observer, None)
    }

    #[cfg(feature = "search-stats")]
    pub fn search_depth_with_stats(
        &self,
        board: &mut Board,
        depth: u8,
    ) -> (SearchResult, SearchStats) {
        let mut stats = SearchStats::default();
        let result = self.search_depth_inner(board, depth, &mut stats, None);

        (result, stats)
    }

    fn search_depth_timed(
        &self,
        board: &mut Board,
        depth: u8,
        timer: &TimeManager,
    ) -> (SearchResult, bool) {
        let mut observer = NoStats;
        let result = self.search_depth_inner(board, depth, &mut observer, Some(timer));

        (result, !timer.should_stop())
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

        let result = search.search_default(&mut board);

        assert!(result.best_move.is_some());
        assert_eq!(board.us(), crate::board::piece::Color::White);
        assert_eq!(board.history.len(), 0);
    }

    #[test]
    fn chooses_free_material_at_depth_two() {
        let mut board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let search = Search::new();

        let result = search.search_default(&mut board);

        let best_move = result.best_move.unwrap();
        assert_eq!(best_move.from(), Square::E4);
        assert_eq!(best_move.to(), Square::E5);
    }

    #[test]
    fn finds_move_in_exposed_black_king_position() {
        let mut board =
            Board::from_fen("kr3b1r/p5pp/p1Qp4/3P4/1P6/P1R5/2P2PPP/2K1R3 b - - 2 23").unwrap();
        let search = Search::new();

        let result = search.search_default(&mut board);

        assert_eq!(result.best_move.unwrap().to_uci(), "b8b7");
    }

    #[test]
    fn finds_move_when_only_terrible_moves_are_available() {
        let mut board = Board::from_fen("2R5/7p/1p2pQp1/8/5k2/P7/1P3PPP/4K2R b K - 2 41").unwrap();
        let search = Search::new();

        let result = search.search_default(&mut board);

        assert!(result.best_move.is_some());
    }

    #[test]
    fn iterative_depth_returns_fixed_depth_result() {
        let mut iterative_board = Board::from_fen("4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1").unwrap();
        let mut fixed_board = iterative_board.clone();
        let search = Search::new();

        let iterative = search.search(&mut iterative_board, SearchLimit::Depth(3));
        let fixed = search.search_depth(&mut fixed_board, 3);

        assert_eq!(iterative, fixed);
        assert_eq!(iterative_board.history.len(), 0);
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
}
