use anton::{board::Board, movegen::MoveGenerator};

fn perft(board: &mut Board, depth: u8, mg: &MoveGenerator) -> u64 {
    let mut leaf_nodes = 0;
    if depth == 0 {
        return 1;
    }

    let ml = mg.gen_moves(board);

    for i in 0..ml.len() {
        let m = ml.get(i);

        if board.make(m, mg) {
            leaf_nodes += perft(board, depth - 1, mg);
            board.unmake();
        }
    }

    leaf_nodes
}

fn run_perft(fen: &str, nodes: &[(u8, u64)]) {
    let Ok(mut board) = Board::from_fen(fen) else {
        panic!("Invalid FEN: {fen}");
    };
    let mg = MoveGenerator::new();

    for &(depth, expected) in nodes {
        assert_eq!(
            perft(&mut board, depth, &mg),
            expected,
            "{fen} at depth {depth}"
        );
    }
}

fn run_perftsuite_case(case_number: usize) {
    let line = include_str!("perftsuite.epd")
        .lines()
        .nth(case_number - 1)
        .unwrap_or_else(|| panic!("Missing perftsuite case {case_number}"));
    let (fen, nodes) = parse_perftsuite_line(line);

    run_perft(fen, &nodes);
}

fn parse_perftsuite_line(line: &str) -> (&str, Vec<(u8, u64)>) {
    let mut parts = line.split(" ;");
    let fen = parts
        .next()
        .unwrap_or_else(|| panic!("Missing FEN in {line}"));
    let nodes = parts
        .map(|part| {
            let (depth, nodes) = part
                .split_once(' ')
                .unwrap_or_else(|| panic!("Invalid perft data: {part}"));
            let depth = depth
                .strip_prefix('D')
                .unwrap_or_else(|| panic!("Invalid perft depth: {depth}"))
                .parse()
                .unwrap_or_else(|_| panic!("Invalid perft depth: {depth}"));
            let nodes = nodes
                .parse()
                .unwrap_or_else(|_| panic!("Invalid perft nodes: {nodes}"));

            (depth, nodes)
        })
        .collect();

    (fen, nodes)
}

macro_rules! perftsuite_cases {
    ($($name:ident: $case_number:literal,)*) => {
        $(
            #[test]
            #[ignore]
            fn $name() {
                run_perftsuite_case($case_number);
            }
        )*
    };
}

    perftsuite_cases! {
        perft_1: 1,
        perft_2: 2,
        perft_3: 3,
        perft_4: 4,
        perft_5: 5,
        perft_6: 6,
        perft_7: 7,
        perft_8: 8,
        perft_9: 9,
        perft_10: 10,
        perft_11: 11,
        perft_12: 12,
        perft_13: 13,
        perft_14: 14,
        perft_15: 15,
        perft_16: 16,
        perft_17: 17,
        perft_18: 18,
        perft_19: 19,
        perft_20: 20,
        perft_21: 21,
        perft_22: 22,
        perft_23: 23,
        perft_24: 24,
        perft_25: 25,
        perft_26: 26,
        perft_27: 27,
        perft_28: 28,
        perft_29: 29,
        perft_30: 30,
        perft_31: 31,
        perft_32: 32,
        perft_33: 33,
        perft_34: 34,
        perft_35: 35,
        perft_36: 36,
        perft_37: 37,
        perft_38: 38,
        perft_39: 39,
        perft_40: 40,
        perft_41: 41,
        perft_42: 42,
        perft_43: 43,
        perft_44: 44,
        perft_45: 45,
        perft_46: 46,
        perft_47: 47,
        perft_48: 48,
        perft_49: 49,
        perft_50: 50,
        perft_51: 51,
        perft_52: 52,
        perft_53: 53,
        perft_54: 54,
        perft_55: 55,
        perft_56: 56,
        perft_57: 57,
        perft_58: 58,
        perft_59: 59,
        perft_60: 60,
        perft_61: 61,
        perft_62: 62,
        perft_63: 63,
        perft_64: 64,
        perft_65: 65,
        perft_66: 66,
        perft_67: 67,
        perft_68: 68,
        perft_69: 69,
        perft_70: 70,
        perft_71: 71,
        perft_72: 72,
        perft_73: 73,
        perft_74: 74,
        perft_75: 75,
        perft_76: 76,
        perft_77: 77,
        perft_78: 78,
        perft_79: 79,
        perft_80: 80,
        perft_81: 81,
        perft_82: 82,
        perft_83: 83,
        perft_84: 84,
        perft_85: 85,
        perft_86: 86,
        perft_87: 87,
        perft_88: 88,
        perft_89: 89,
        perft_90: 90,
        perft_91: 91,
        perft_92: 92,
        perft_93: 93,
        perft_94: 94,
        perft_95: 95,
        perft_96: 96,
        perft_97: 97,
        perft_98: 98,
        perft_99: 99,
        perft_100: 100,
        perft_101: 101,
        perft_102: 102,
        perft_103: 103,
        perft_104: 104,
        perft_105: 105,
        perft_106: 106,
        perft_107: 107,
        perft_108: 108,
        perft_109: 109,
        perft_110: 110,
        perft_111: 111,
        perft_112: 112,
        perft_113: 113,
        perft_114: 114,
        perft_115: 115,
        perft_116: 116,
        perft_117: 117,
        perft_118: 118,
        perft_119: 119,
        perft_120: 120,
        perft_121: 121,
        perft_122: 122,
        perft_123: 123,
        perft_124: 124,
        perft_125: 125,
        perft_126: 126,
        perft_127: 127,
        perft_128: 128,
    }
