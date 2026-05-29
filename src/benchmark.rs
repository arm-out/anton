use std::time::Instant;

use crate::{
    board::Board,
    search::{DEFAULT_TT_SIZE_MB, Search, SearchLimit},
};

const FEN_DEFAULT: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const BENCH_DEPTH: u8 = 7;

pub fn run() {
    let start = Instant::now();

    let mut board = Board::from_fen(FEN_DEFAULT).expect("bench FEN should be valid");
    let mut search = Search::new(DEFAULT_TT_SIZE_MB);
    let result = search.search(&mut board, SearchLimit::Depth(BENCH_DEPTH));

    let nodes = result.stats.nodes + result.stats.qnodes;

    let elapsed = start.elapsed();
    let nps = if elapsed.as_nanos() == 0 {
        0
    } else {
        (u128::from(nodes) * 1_000_000_000 / elapsed.as_nanos()) as u64
    };

    println!("{nodes} nodes {nps} nps");
}
