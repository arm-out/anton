use anton::{board::Board, movegen::MoveGenerator};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

struct PerftPosition {
    name: &'static str,
    fen: &'static str,
    depth: u8,
    nodes: u64,
}

const POSITIONS: &[PerftPosition] = &[
    PerftPosition {
        name: "startpos_d5",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 5,
        nodes: 4_865_609,
    },
    PerftPosition {
        name: "kiwipete_d4",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 4,
        nodes: 4_085_603,
    },
    PerftPosition {
        name: "position3_d5",
        fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        depth: 5,
        nodes: 674_624,
    },
    PerftPosition {
        name: "position4_d4",
        fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        depth: 4,
        nodes: 422_333,
    },
];

fn perft(board: &mut Board, depth: u8, movegen: &MoveGenerator) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = movegen.gen_moves(board);
    let mut nodes = 0;

    for i in 0..moves.len() {
        let m = moves.get(i);

        if board.make(m, movegen) {
            nodes += perft(board, depth - 1, movegen);
            board.unmake();
        }
    }

    nodes
}

fn bench_perft(c: &mut Criterion) {
    let movegen = MoveGenerator::new();
    let mut group = c.benchmark_group("movegen/perft");

    for position in POSITIONS {
        group.throughput(Throughput::Elements(position.nodes));

        group.bench_function(position.name, |b| {
            b.iter(|| {
                let mut board = Board::from_fen(position.fen).unwrap();
                let nodes = perft(&mut board, black_box(position.depth), &movegen);

                assert_eq!(nodes, position.nodes);
                black_box(nodes)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_perft);
criterion_main!(benches);
