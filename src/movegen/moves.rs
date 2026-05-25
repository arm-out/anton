use crate::board::square::Square;

// --------- MOVE DATA ---------
// 0000  000000      000000
// FLAGS FROM_SQUARE TO_SQUARE
// -----------------------------
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Move(pub u16);

// 0101 001001 000000

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

    pub fn to_uci(&self) -> String {
        let promotion = match self.kind() {
            MoveType::NPromotion | MoveType::NPromoCapture => "n",
            MoveType::BPromotion | MoveType::BPromoCapture => "b",
            MoveType::RPromotion | MoveType::RPromoCapture => "r",
            MoveType::QPromotion | MoveType::QPromoCapture => "q",
            _ => "",
        };

        format!(
            "{}{}{}",
            self.from().to_string().to_ascii_lowercase(),
            self.to().to_string().to_ascii_lowercase(),
            promotion
        )
    }
}

impl MoveType {
    pub fn from(flags: u8) -> Self {
        unsafe { std::mem::transmute(flags) }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let from = self.from();
        let to = self.to();
        let kind = self.kind();

        write!(f, "{} {} {}", from, to, kind)
    }
}

impl std::fmt::Display for MoveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            MoveType::Quiet => String::from("Quiet"),
            MoveType::DoublePawnPush => String::from("DoublePawnPush"),
            MoveType::CastleKingside => String::from("CastleKingside"),
            MoveType::CastleQueenside => String::from("CastleQueenside"),
            MoveType::Capture => String::from("Capture"),
            MoveType::EnPassant => String::from("EnPassant"),
            MoveType::NPromotion => String::from("NPromotion"),
            MoveType::BPromotion => String::from("BPromotion"),
            MoveType::RPromotion => String::from("RPromotion"),
            MoveType::QPromotion => String::from("QPromotion"),
            MoveType::NPromoCapture => String::from("NPromoCapture"),
            MoveType::BPromoCapture => String::from("BPromoCapture"),
            MoveType::RPromoCapture => String::from("RPromoCapture"),
            MoveType::QPromoCapture => String::from("QPromoCapture"),
        };

        write!(f, "{}", kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_quiet_move_as_uci() {
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);

        assert_eq!(m.to_uci(), "e2e4");
    }

    #[test]
    fn formats_promotion_move_as_uci() {
        let m = Move::new(Square::E7, Square::E8, MoveType::QPromotion);

        assert_eq!(m.to_uci(), "e7e8q");
    }
}
