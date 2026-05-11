use std::ops::{Index, IndexMut};

#[repr(u8)]
#[derive(Default, Copy, Clone, Debug)]
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
    pub fn color(self) -> Color {
        unsafe { std::mem::transmute((self as u8) & 1) }
    }

    pub fn piece_type(self) -> PieceType {
        unsafe { std::mem::transmute((self as u8) >> 1) }
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

impl Piece {
    pub const COUNT: usize = 12;
}

impl PieceType {
    pub const COUNT: usize = 6;
}

impl Color {
    pub const COUNT: usize = 2;
}
