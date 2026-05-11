#[repr(u8)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[repr(u8)]
pub enum Color {
    White,
    Black,
}
