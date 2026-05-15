use crate::board::{bitboard::Bitboard, square::Square};

pub struct Magic {
    pub mask: Bitboard,
    pub shift: u8,
    pub offset: u64,
    pub magic: u64,
}

// pub const ROOK_MAGICS: [Magic; Square::COUNT] = todo!();
// pub const BISHOP_MAGICS: [Magic; Square::COUNT] = todo!();
