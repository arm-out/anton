use crate::board::square::Square;

// --------- MOVE DATA ---------
// 0000  000000      000000
// FLAGS FROM_SQUARE TO_SQUARE
// -----------------------------
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Move(u16);

// 0         0       0        0
// Promotion Capture Special1 Special0
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MoveType {
    Quiet = 0b0000,
    DoublePawnPush = 0b0001,
    CastleKingside = 0b0010,
    CastleQueenside = 0b0011,

    Capture = 0b0100,
    EnPassant = 0b0101,

    NPromotion = 0b1000,
    BPromotion = 0b1001,
    RPromotion = 0b1010,
    QPromotion = 0b1011,

    NPromoCapture = 0b1100,
    BPromoCapture = 0b1101,
    RPromoCapture = 0b1110,
    QPromoCapture = 0b1111,
}

pub const PROMO_TYPES: [MoveType; 4] = [
    MoveType::NPromotion,
    MoveType::BPromotion,
    MoveType::RPromotion,
    MoveType::QPromotion,
];

pub const PROMO_CAPTURES: [MoveType; 4] = [
    MoveType::NPromoCapture,
    MoveType::BPromoCapture,
    MoveType::RPromoCapture,
    MoveType::QPromoCapture,
];

impl Move {
    pub fn new(from: Square, to: Square, flags: MoveType) -> Self {
        Self((to as u16) | ((from as u16) << 6) | (flags as u16) << 12)
    }

    pub fn is_capture(&self) -> bool {
        unsafe { std::mem::transmute((self.0 & 0b0100_0000_0000_0000) != 0) }
    }

    pub fn kind(&self) -> MoveType {
        MoveType::from(((self.0 & 0b1111_000000_000000) >> 12) as u8)
    }

    pub fn from(&self) -> Square {
        unsafe { std::mem::transmute(((self.0 & 0b0000_111111_000000) >> 6) as u8) }
    }

    pub fn to(&self) -> Square {
        unsafe { std::mem::transmute((self.0 & 0b0000_000000_111111) as u8) }
    }
}

impl MoveType {
    pub fn from(flags: u8) -> Self {
        unsafe { std::mem::transmute(flags) }
    }
}
