use std::ops::{Index, IndexMut, Not};

#[repr(u8)]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
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

impl Piece {
    // Transmute memory trick from https://github.com/codedeliveryservice/Reckless
    #[inline]
    pub fn color(self) -> Color {
        unsafe { std::mem::transmute((self as u8) & 1) }
    }

    #[inline]
    pub fn piece_type(self) -> PieceType {
        unsafe { std::mem::transmute((self as u8) >> 1) }
    }

    #[inline]
    pub fn from_index(index: usize) -> Self {
        unsafe { std::mem::transmute(index as u8) }
    }
}

impl<T> Index<Piece> for [T] {
    type Output = T;

    fn index(&self, piece: Piece) -> &Self::Output {
        &self[piece as usize]
    }
}

impl<T> IndexMut<Piece> for [T] {
    fn index_mut(&mut self, piece: Piece) -> &mut Self::Output {
        &mut self[piece as usize]
    }
}

#[repr(u8)]
#[derive(Default, PartialEq, Debug)]
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

impl<T> Index<PieceType> for [T] {
    type Output = T;

    fn index(&self, piece_type: PieceType) -> &Self::Output {
        &self[piece_type as usize]
    }
}

impl<T> IndexMut<PieceType> for [T] {
    fn index_mut(&mut self, piece_type: PieceType) -> &mut Self::Output {
        &mut self[piece_type as usize]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl<T> Index<Color> for [T] {
    type Output = T;

    fn index(&self, color: Color) -> &Self::Output {
        &self[color as usize]
    }
}

impl<T> IndexMut<Color> for [T] {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        &mut self[color as usize]
    }
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        unsafe { std::mem::transmute((self as u8) ^ 1) }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_color() {
        assert_eq!(Piece::WhitePawn.color(), Color::White);
        assert_eq!(Piece::BlackPawn.color(), Color::Black);
        assert_eq!(Piece::WhiteKnight.color(), Color::White);
        assert_eq!(Piece::BlackKnight.color(), Color::Black);
        assert_eq!(Piece::WhiteBishop.color(), Color::White);
        assert_eq!(Piece::BlackBishop.color(), Color::Black);
        assert_eq!(Piece::WhiteRook.color(), Color::White);
        assert_eq!(Piece::BlackRook.color(), Color::Black);
        assert_eq!(Piece::WhiteQueen.color(), Color::White);
        assert_eq!(Piece::BlackQueen.color(), Color::Black);
        assert_eq!(Piece::WhiteKing.color(), Color::White);
        assert_eq!(Piece::BlackKing.color(), Color::Black);
    }

    #[test]
    fn test_piece_piece_type() {
        assert_eq!(Piece::WhitePawn.piece_type(), PieceType::Pawn);
        assert_eq!(Piece::BlackPawn.piece_type(), PieceType::Pawn);
        assert_eq!(Piece::WhiteKnight.piece_type(), PieceType::Knight);
        assert_eq!(Piece::BlackKnight.piece_type(), PieceType::Knight);
        assert_eq!(Piece::WhiteBishop.piece_type(), PieceType::Bishop);
        assert_eq!(Piece::BlackBishop.piece_type(), PieceType::Bishop);
        assert_eq!(Piece::WhiteRook.piece_type(), PieceType::Rook);
        assert_eq!(Piece::BlackRook.piece_type(), PieceType::Rook);
        assert_eq!(Piece::WhiteQueen.piece_type(), PieceType::Queen);
        assert_eq!(Piece::BlackQueen.piece_type(), PieceType::Queen);
        assert_eq!(Piece::WhiteKing.piece_type(), PieceType::King);
        assert_eq!(Piece::BlackKing.piece_type(), PieceType::King);
    }

    #[test]
    fn test_piece_from_index() {
        assert_eq!(Piece::from_index(0), Piece::WhitePawn);
        assert_eq!(Piece::from_index(1), Piece::BlackPawn);
        assert_eq!(Piece::from_index(2), Piece::WhiteKnight);
        assert_eq!(Piece::from_index(3), Piece::BlackKnight);
        assert_eq!(Piece::from_index(4), Piece::WhiteBishop);
        assert_eq!(Piece::from_index(5), Piece::BlackBishop);
        assert_eq!(Piece::from_index(6), Piece::WhiteRook);
        assert_eq!(Piece::from_index(7), Piece::BlackRook);
        assert_eq!(Piece::from_index(8), Piece::WhiteQueen);
        assert_eq!(Piece::from_index(9), Piece::BlackQueen);
        assert_eq!(Piece::from_index(10), Piece::WhiteKing);
        assert_eq!(Piece::from_index(11), Piece::BlackKing);
    }

    #[test]
    fn test_color_not() {
        assert_eq!(!Color::White, Color::Black);
        assert_eq!(!Color::Black, Color::White);
    }
}
