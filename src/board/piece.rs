#[repr(u8)]
#[derive(Default)]
pub enum Piece {
    WhitePawn,
    BlackPawn,
    WhiteKnight,
    BlackKnight,
    WhiteBishop,
    BlackBishop,
    WhiteRook,
    BlackRook,
    WhiteQueen,
    BlackQueen,
    WhiteKing,
    BlackKing,
    #[default]
    None,
}

#[repr(u8)]
#[derive(Default)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    #[default]
    None,
}

#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Piece {
    pub const COUNT: usize = 12;
}

impl PieceType {
    pub const COUNT: usize = 6;
}

impl Color {
    pub const COUNT: usize = 2;
}
