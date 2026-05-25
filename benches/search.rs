use anton::{board::Board, search::Search};
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

fn bench_search(c: &mut Criterion) {
    let search = Search::new();
    let mut group = c.benchmark_group("search/fixed_depth");

    for position in POSITIONS {
        let template = Board::from_fen(position.fen).unwrap();
        let mut board = template.clone();
        let (_, expected_stats) = search.search_depth_with_stats(&mut board, position.depth);

        group.throughput(Throughput::Elements(expected_stats.nodes));

        group.bench_function(position.name, |b| {
            b.iter(|| {
                let mut board = template.clone();
                let (result, stats) = search.search_depth_with_stats(&mut board, black_box(position.depth));

                assert!(result.best_move.is_some());
                assert_eq!(stats.nodes, expected_stats.nodes);
                assert_eq!(stats.leaves, expected_stats.leaves);
                assert_eq!(stats.beta_cutoffs, expected_stats.beta_cutoffs);

                black_box((result, stats))
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
