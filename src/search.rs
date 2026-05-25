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

    pub fn search(&self, board: &mut Board) -> SearchResult {
        self.search_depth(board, SEARCH_DEPTH)
    }

    pub fn search_depth(&self, board: &mut Board, depth: u8) -> SearchResult {
        let mut observer = NoStats;

        self.search_depth_inner(board, depth, &mut observer)
    }

    #[cfg(feature = "search-stats")]
    pub fn search_depth_with_stats(
        &self,
        board: &mut Board,
        depth: u8,
    ) -> (SearchResult, SearchStats) {
        let mut stats = SearchStats::default();
        let result = self.search_depth_inner(board, depth, &mut stats);

        (result, stats)
    }

    fn search_depth_inner<O: SearchObserver>(
        &self,
        board: &mut Board,
        depth: u8,
        observer: &mut O,
    ) -> SearchResult {
        observer.root();

        if depth == 0 {
            observer.leaf();

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
            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, depth - 1, -beta, -alpha, observer);
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

    fn negamax<O: SearchObserver>(
        &self,
        board: &mut Board,
        depth: u8,
        mut alpha: Score,
        beta: Score,
        observer: &mut O,
    ) -> Score {
        observer.node();

        if depth == 0 {
            observer.leaf();
            return board.state.evaluation.score(board.us());
        }

        let mut best_score = -INF;
        let moves = self.movegen.gen_moves(board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if !board.make(m, &self.movegen) {
                continue;
            }

            let score = -self.negamax(board, depth - 1, -beta, -alpha, observer);
            board.unmake();

            best_score = best_score.max(score);
            alpha = alpha.max(score);

            if alpha >= beta {
                observer.beta_cutoff();
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
