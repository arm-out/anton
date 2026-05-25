use anton::{board::Board, movegen::MoveGenerator};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

const POSITIONS: &[(&str, &str)] = &[
    (
        "startpos",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    ),
    (
        "kiwipete",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    ),
    (
        "castling",
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
    ),
    (
        "knights",
        "8/1n4N1/2k5/8/8/5K2/1N4n1/8 w - - 0 1",
    ),
    (
        "endgame",
        "8/8/8/3k4/8/4K3/8/8 w - - 0 1",
    ),
];

fn bench_gen_moves(c: &mut Criterion) {
    let movegen = MoveGenerator::new();
    let mut group = c.benchmark_group("movegen/gen_moves");
    group.throughput(Throughput::Elements(1));

    for &(name, fen) in POSITIONS {
        let board = Board::from_fen(fen).unwrap();

        group.bench_function(name, |b| {
            b.iter(|| {
                let moves = movegen.gen_moves(black_box(&board));
                black_box(moves.len())
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_gen_moves);
criterion_main!(benches);
