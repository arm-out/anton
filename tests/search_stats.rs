use anton::{
    board::Board,
    search::{DEFAULT_TT_SIZE_MB, Search},
};

struct SearchStatsCase {
    name: &'static str,
    fen: &'static str,
    depth: u8,
    perft_nodes: u64,
}

const CASES: &[SearchStatsCase] = &[
    SearchStatsCase {
        name: "startpos_d7",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 7,
        perft_nodes: 3_195_901_860,
    },
    SearchStatsCase {
        name: "kiwipete_d6",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 6,
        perft_nodes: 8_031_647_685,
    },
    SearchStatsCase {
        name: "position3_d7",
        fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ",
        depth: 7,
        perft_nodes: 178_633_661,
    },
    SearchStatsCase {
        name: "position4_d7",
        fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        depth: 7,
        perft_nodes: 706_045_033,
    },
    SearchStatsCase {
        name: "steven_d6",
        fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        depth: 6,
        perft_nodes: 6_923_051_137,
    },
];

#[test]
#[ignore]
fn print_search_stats() {
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
        let mut search = Search::new(DEFAULT_TT_SIZE_MB);
        let stats = search
            .search(&mut board, anton::search::SearchLimit::Depth(case.depth))
            .stats;

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
