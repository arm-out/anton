// --------- MOVE DATA ---------
// 0000  000000      000000
// FLAGS FROM_SQUARE TO_SQUARE
// -----------------------------
pub struct Move(u16);

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

impl Move {
    pub fn new(from: Square, to: Square, flags: MoveType) -> Self {
        Self((to as u16) | ((from as u16) << 6) | (flags as u16) << 12)
    }
}
