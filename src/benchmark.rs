use std::time::{Duration, Instant};

use crate::{
    board::Board,
    search::{DEFAULT_TT_SIZE_MB, Search, SearchLimit},
};

struct BenchPosition {
    fen: &'static str,
    depth: u8,
}

const POSITIONS: &[BenchPosition] = &[
    BenchPosition {
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 5,
    },
    BenchPosition {
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 4,
    },
    BenchPosition {
        fen: "4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1",
        depth: 5,
    },
    BenchPosition {
        fen: "8/8/8/3k4/8/4K3/8/8 w - - 0 1",
        depth: 6,
    },
];

pub fn run() {
    let mut nodes = 0;
    let mut elapsed = Duration::from_secs(0);

    for position in POSITIONS {
        let mut board = Board::from_fen(position.fen).expect("bench FEN should be valid");
        let mut search = Search::new(DEFAULT_TT_SIZE_MB);
        let start = Instant::now();
        let result = search.search(&mut board, SearchLimit::Depth(position.depth));

        elapsed += start.elapsed();
        nodes += result.stats.nodes + result.stats.qnodes;
    }

    let nps = (u128::from(nodes) * 1_000_000_000 / elapsed.as_nanos()) as u64;

    println!("{nodes} nodes {nps} nps")
}
