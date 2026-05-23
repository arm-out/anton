use std::ops::Index;

use crate::board::{piece::Color, square::Square};

// KQkq
#[derive(Copy, Clone)]
pub enum CastlingKind {
    WhiteKingside = 0b1000,
    WhiteQueenside = 0b0100,
    BlackKingside = 0b0010,
    BlackQueenside = 0b0001,
}

impl CastlingKind {
    pub const KIND_BY_COLOR: [[CastlingKind; 2]; Color::COUNT] = [
        [CastlingKind::WhiteKingside, CastlingKind::WhiteQueenside],
        [CastlingKind::BlackKingside, CastlingKind::BlackQueenside],
    ];

    pub fn castling_destination(self) -> Square {
        match self {
            CastlingKind::WhiteKingside => Square::G1,
            CastlingKind::WhiteQueenside => Square::C1,
            CastlingKind::BlackKingside => Square::G8,
            CastlingKind::BlackQueenside => Square::C8,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CastlingPerms {
    pub raw: u8,
}

impl CastlingPerms {
    pub const fn raw(self) -> u8 {
        self.raw
    }

    pub fn is_allowed(self, kind: CastlingKind) -> bool {
        (self.raw & kind as u8) != 0
    }
}

impl<T> Index<CastlingPerms> for [T] {
    type Output = T;
    fn index(&self, castling_perms: CastlingPerms) -> &Self::Output {
        &self[castling_perms.raw as usize]
    }
}
