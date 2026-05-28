use anton::{board::Board, search::Search};

struct SearchStatsCase {
    name: &'static str,
    fen: &'static str,
    depth: u8,
    perft_nodes: u64,
}

const CASES: &[SearchStatsCase] = &[
    SearchStatsCase {
        name: "startpos_d5",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 5,
        perft_nodes: 4_865_609,
    },
    SearchStatsCase {
        name: "kiwipete_d4",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 4,
        perft_nodes: 4_085_603,
    },
    SearchStatsCase {
        name: "castling_d5",
        fen: "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        depth: 5,
        perft_nodes: 7_594_526,
    },
    SearchStatsCase {
        name: "rook_endgame_d4",
        fen: "R6r/8/8/2K5/5k2/8/8/r6R w - - 0 1",
        depth: 4,
        perft_nodes: 771_461,
    },
    SearchStatsCase {
        name: "bishop_endgame_d5",
        fen: "8/8/1B6/7b/7k/8/2B1b3/7K w - - 0 1",
        depth: 5,
        perft_nodes: 1_713_368,
    },
];

#[test]
#[ignore]
fn print_search_stats() {
    let search = Search::new();

    println!(
        "{:<18} {:>5} {:>12} {:>12} {:>12} {:>12} {:>12} {:>10} {:>10} {:>10}",
        "case",
        "depth",
        "perft",
        "nodes",
        "qnodes",
        "leaves",
        "cutoffs",
        "searched%",
        "leaf%",
        "cutoff%"
    );

    for case in CASES {
        let mut board = Board::from_fen(case.fen).unwrap();
        let stats = search.search_depth(&mut board, case.depth).stats;

        let total_nodes = stats.nodes + stats.qnodes;
        let searched_pct = total_nodes as f64 * 100.0 / case.perft_nodes as f64;
        let leaf_pct = stats.leaves as f64 * 100.0 / total_nodes as f64;
        let cutoff_pct = stats.beta_cutoffs as f64 * 100.0 / total_nodes as f64;

        println!(
            "{:<18} {:>5} {:>12} {:>12} {:>12} {:>12} {:>12} {:>9.3}% {:>9.3}% {:>9.3}%",
            case.name,
            case.depth,
            case.perft_nodes,
            stats.nodes,
            stats.qnodes,
            stats.leaves,
            stats.beta_cutoffs,
            searched_pct,
            leaf_pct,
            cutoff_pct
        );
    }
}
