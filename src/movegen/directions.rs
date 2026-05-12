use crate::board::{
    bitboard::Bitboard,
    square::{File, Rank},
};

pub struct MoveShift {
    pub shift: i8,
    pub exclude: Bitboard,
}

//   northwest    north   northeast
//   noWe         nort         noEa
//           +7    +8    +9
//               \  |  /
//   west    -1 <-  0 -> +1    east
//               /  |  \
//           -9    -8    -7
//   soWe         sout         soEa
//   southwest    south   southeast
//
// https://www.chessprogramming.org/Direction

pub const PAWN_SHIFT_WHITE: [MoveShift; 2] = [
    MoveShift {
        shift: 7,
        exclude: Bitboard::from_file(File::A),
    },
    MoveShift {
        shift: 9,
        exclude: Bitboard::from_file(File::H),
    },
];

pub const PAWN_SHIFT_BLACK: [MoveShift; 2] = [
    MoveShift {
        shift: -7,
        exclude: Bitboard::from_file(File::H),
    },
    MoveShift {
        shift: -9,
        exclude: Bitboard::from_file(File::A),
    },
];

pub const KING_SHIFTS: [MoveShift; 8] = [
    MoveShift {
        shift: 8,
        exclude: Bitboard::from_rank(Rank::R8),
    },
    MoveShift {
        shift: 9,
        exclude: Bitboard::from_rank(Rank::R8).union(Bitboard::from_file(File::H)),
    },
    MoveShift {
        shift: 7,
        exclude: Bitboard::from_rank(Rank::R8).union(Bitboard::from_file(File::A)),
    },
    MoveShift {
        shift: 1,
        exclude: Bitboard::from_file(File::H),
    },
    MoveShift {
        shift: -1,
        exclude: Bitboard::from_file(File::A),
    },
    MoveShift {
        shift: -8,
        exclude: Bitboard::from_rank(Rank::R1),
    },
    MoveShift {
        shift: -9,
        exclude: Bitboard::from_rank(Rank::R1).union(Bitboard::from_file(File::A)),
    },
    MoveShift {
        shift: -7,
        exclude: Bitboard::from_rank(Rank::R1).union(Bitboard::from_file(File::H)),
    },
];

//         noNoWe    noNoEa
//             +15  +17
//              |     |
// noWeWe  +6 __|     |__+10  noEaEa
//               \   /
//                >0<
//            __ /   \ __
// soWeWe -10   |     |   -6  soEaEa
//              |     |
//             -17  -15
//         soSoWe    soSoEa
//
// https://www.chessprogramming.org/Direction

pub const KNIGHT_SHIFTS: [MoveShift; 8] = [
    MoveShift {
        shift: 15,
        exclude: Bitboard::from_file(File::A)
            .union(Bitboard::from_rank(Rank::R7))
            .union(Bitboard::from_rank(Rank::R8)),
    },
    MoveShift {
        shift: 17,
        exclude: Bitboard::from_file(File::H)
            .union(Bitboard::from_rank(Rank::R7))
            .union(Bitboard::from_rank(Rank::R8)),
    },
    MoveShift {
        shift: 6,
        exclude: Bitboard::from_file(File::A)
            .union(Bitboard::from_file(File::B))
            .union(Bitboard::from_rank(Rank::R8)),
    },
    MoveShift {
        shift: 10,
        exclude: Bitboard::from_file(File::G)
            .union(Bitboard::from_file(File::H))
            .union(Bitboard::from_rank(Rank::R8)),
    },
    MoveShift {
        shift: -15,
        exclude: Bitboard::from_file(File::H)
            .union(Bitboard::from_rank(Rank::R1))
            .union(Bitboard::from_rank(Rank::R2)),
    },
    MoveShift {
        shift: -17,
        exclude: Bitboard::from_file(File::A)
            .union(Bitboard::from_rank(Rank::R1))
            .union(Bitboard::from_rank(Rank::R2)),
    },
    MoveShift {
        shift: -6,
        exclude: Bitboard::from_file(File::G)
            .union(Bitboard::from_file(File::H))
            .union(Bitboard::from_rank(Rank::R1)),
    },
    MoveShift {
        shift: -10,
        exclude: Bitboard::from_file(File::A)
            .union(Bitboard::from_file(File::B))
            .union(Bitboard::from_rank(Rank::R1)),
    },
];
