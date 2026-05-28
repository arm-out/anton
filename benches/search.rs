use std::time::Duration;

use anton::{
    board::Board,
    search::{Search, SearchLimit},
};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

struct SearchPosition {
    name: &'static str,
    fen: &'static str,
    depth: u8,
}

const POSITIONS: &[SearchPosition] = &[
    SearchPosition {
        name: "startpos_d4",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 4,
    },
    SearchPosition {
        name: "kiwipete_d3",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 3,
    },
    SearchPosition {
        name: "tactical_d4",
        fen: "4k3/8/8/4q3/4R3/8/8/4K3 w - - 0 1",
        depth: 4,
    },
    SearchPosition {
        name: "endgame_d5",
        fen: "8/8/8/3k4/8/4K3/8/8 w - - 0 1",
        depth: 5,
    },
];

struct TimedSearchPosition {
    name: &'static str,
    fen: &'static str,
    movetime: Duration,
}

const TIMED_POSITIONS: &[TimedSearchPosition] = &[
    TimedSearchPosition {
        name: "startpos_25ms",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        movetime: Duration::from_millis(25),
    },
    TimedSearchPosition {
        name: "kiwipete_25ms",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        movetime: Duration::from_millis(25),
    },
];

fn bench_search(c: &mut Criterion) {
    let search = Search::new();
    let mut group = c.benchmark_group("search/fixed_depth");

    for position in POSITIONS {
        let template = Board::from_fen(position.fen).unwrap();
        let mut board = template.clone();
        let expected_stats = search.search_depth(&mut board, position.depth).stats;

        group.throughput(Throughput::Elements(
            expected_stats.nodes + expected_stats.qnodes,
        ));

        group.bench_function(position.name, |b| {
            b.iter(|| {
                let mut board = template.clone();
                let result = search.search_depth(&mut board, black_box(position.depth));
                let stats = result.stats;

                assert!(result.best_move.is_some());
                assert_eq!(stats.nodes, expected_stats.nodes);
                assert_eq!(stats.qnodes, expected_stats.qnodes);
                assert_eq!(stats.leaves, expected_stats.leaves);
                assert_eq!(stats.beta_cutoffs, expected_stats.beta_cutoffs);

                black_box((result, stats))
            });
        });
    }

    group.finish();
}

fn bench_timed_search(c: &mut Criterion) {
    let search = Search::new();
    let mut group = c.benchmark_group("search/timed");
    group.sample_size(10);

    for position in TIMED_POSITIONS {
        let template = Board::from_fen(position.fen).unwrap();
        let mut board = template.clone();
        let baseline = search.search(&mut board, SearchLimit::MoveTime(position.movetime));

        eprintln!(
            "{}: depth={} nodes={} qnodes={} leaves={} cutoffs={} score={}",
            position.name,
            baseline.depth,
            baseline.stats.nodes,
            baseline.stats.qnodes,
            baseline.stats.leaves,
            baseline.stats.beta_cutoffs,
            baseline.score
        );

        group.throughput(Throughput::Elements(
            (baseline.stats.nodes + baseline.stats.qnodes).max(1),
        ));

        group.bench_function(position.name, |b| {
            b.iter(|| {
                let mut board = template.clone();
                let result = search.search(
                    &mut board,
                    SearchLimit::MoveTime(black_box(position.movetime)),
                );

                assert_eq!(board.history.len(), 0);

                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_search, bench_timed_search);
criterion_main!(benches);
