use std::ops::{Add, Index, IndexMut, Sub};

#[rustfmt::skip]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
// Little Endian Rank-File Mapping (LERF) https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
    None,
}

impl Square {
    pub const COUNT: usize = 64;

    pub fn new(square: u8) -> Self {
        unsafe { std::mem::transmute(square) }
    }

    pub fn rank(self) -> Rank {
        unsafe { std::mem::transmute(self as u8 >> 3) }
    }

    pub fn file(self) -> File {
        unsafe { std::mem::transmute(self as u8 & 7) }
    }

    pub fn from_rank_and_file(rank: u8, file: u8) -> Self {
        Self::new(rank << 3 | file)
    }

    pub fn from_idx(idx: usize) -> Self {
        unsafe { std::mem::transmute(idx as u8) }
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Square::None {
            return write!(f, "None");
        }

        let file = (self.file() as u8 + b'A') as char;
        let rank = (self.rank() as u8 + b'1') as char;
        write!(f, "{}{}", file, rank)
    }
}

impl<T> Index<Square> for [T] {
    type Output = T;

    fn index(&self, square: Square) -> &Self::Output {
        &self[square as usize]
    }
}

impl<T> IndexMut<Square> for [T] {
    fn index_mut(&mut self, square: Square) -> &mut Self::Output {
        &mut self[square as usize]
    }
}

impl Add<u8> for Square {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 + rhs as u8) % 64) }
    }
}

impl Sub<u8> for Square {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 - rhs as u8) % 64) }
    }
}

#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Rank {
    R1, R2, R3, R4, R5, R6, R7, R8,
}

impl Rank {
    pub fn from_num(num: u8) -> Self {
        unsafe { std::mem::transmute(num - 1) }
    }
}

impl Add<u8> for Rank {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 + rhs) % 8) }
    }
}

impl Sub<u8> for Rank {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 + 8 - rhs % 8) % 8) }
    }
}

#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum File {
    A, B, C, D, E, F, G, H,
}

impl File {
    pub fn from_char(c: char) -> Self {
        let c_lower: char = c.to_ascii_lowercase();
        unsafe { std::mem::transmute(c_lower as u8 - b'a') }
    }
}

impl Add<u8> for File {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 + rhs) % 8) }
    }
}

impl Sub<u8> for File {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        unsafe { std::mem::transmute((self as u8 + 8 - rhs % 8) % 8) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rank_and_file() {
        let square = Square::from_rank_and_file(0, 0);
        assert_eq!(square as u8, Square::A1 as u8);
        let square = Square::from_rank_and_file(7, 7);
        assert_eq!(square as u8, Square::H8 as u8);
        let square = Square::from_rank_and_file(3, 4);
        assert_eq!(square as u8, Square::E4 as u8);
        let square = Square::from_rank_and_file(2, 6);
        assert_eq!(square as u8, Square::G3 as u8);
    }

    #[test]
    fn test_new() {
        let square = Square::new(Square::A1 as u8);
        assert_eq!(square as u8, 0 as u8);
        let square = Square::new(Square::H8 as u8);
        assert_eq!(square as u8, 63 as u8);
        let square = Square::new(Square::D3 as u8);
        assert_eq!(square as u8, 19 as u8);
        let square = Square::new(Square::F6 as u8);
        assert_eq!(square as u8, 45 as u8);
    }

    #[test]
    fn test_rank_add() {
        assert_eq!(Rank::R1 + 1, Rank::R2);
        assert_eq!(Rank::R2 + 1, Rank::R3);
        assert_eq!(Rank::R3 + 1, Rank::R4);
        assert_eq!(Rank::R4 + 1, Rank::R5);
        assert_eq!(Rank::R5 + 1, Rank::R6);
        assert_eq!(Rank::R6 + 1, Rank::R7);
        assert_eq!(Rank::R7 + 1, Rank::R8);
        assert_eq!(Rank::R8 + 1, Rank::R1);
        assert_eq!(Rank::R1 + 2, Rank::R3);
    }

    #[test]
    fn test_rank_sub() {
        assert_eq!(Rank::R8 - 1, Rank::R7);
        assert_eq!(Rank::R7 - 1, Rank::R6);
        assert_eq!(Rank::R6 - 1, Rank::R5);
        assert_eq!(Rank::R5 - 1, Rank::R4);
        assert_eq!(Rank::R4 - 1, Rank::R3);
        assert_eq!(Rank::R3 - 1, Rank::R2);
        assert_eq!(Rank::R2 - 1, Rank::R1);
        assert_eq!(Rank::R1 - 1, Rank::R8);
        assert_eq!(Rank::R8 - 2, Rank::R6);
        assert_eq!(Rank::R6 - 2, Rank::R4);
    }

    #[test]
    fn test_file_add() {
        assert_eq!(File::A + 1, File::B);
        assert_eq!(File::B + 1, File::C);
        assert_eq!(File::C + 1, File::D);
        assert_eq!(File::D + 1, File::E);
        assert_eq!(File::E + 1, File::F);
        assert_eq!(File::F + 1, File::G);
        assert_eq!(File::G + 1, File::H);
        assert_eq!(File::H + 1, File::A);
        assert_eq!(File::A + 2, File::C);
    }

    #[test]
    fn test_file_sub() {
        assert_eq!(File::H - 1, File::G);
        assert_eq!(File::G - 1, File::F);
        assert_eq!(File::F - 1, File::E);
        assert_eq!(File::E - 1, File::D);
        assert_eq!(File::D - 1, File::C);
        assert_eq!(File::C - 1, File::B);
        assert_eq!(File::B - 1, File::A);
        assert_eq!(File::A - 1, File::H);
        assert_eq!(File::H - 2, File::F);
        assert_eq!(File::G - 2, File::E);
    }

    #[test]
    fn test_rank_from_num() {
        assert_eq!(Rank::from_num(1), Rank::R1);
        assert_eq!(Rank::from_num(2), Rank::R2);
        assert_eq!(Rank::from_num(3), Rank::R3);
        assert_eq!(Rank::from_num(4), Rank::R4);
        assert_eq!(Rank::from_num(5), Rank::R5);
        assert_eq!(Rank::from_num(6), Rank::R6);
        assert_eq!(Rank::from_num(7), Rank::R7);
        assert_eq!(Rank::from_num(8), Rank::R8);
    }

    #[test]
    fn test_file_from_char() {
        assert_eq!(File::from_char('a'), File::A);
        assert_eq!(File::from_char('b'), File::B);
        assert_eq!(File::from_char('c'), File::C);
        assert_eq!(File::from_char('d'), File::D);
        assert_eq!(File::from_char('e'), File::E);
        assert_eq!(File::from_char('f'), File::F);
        assert_eq!(File::from_char('g'), File::G);
        assert_eq!(File::from_char('h'), File::H);
        assert_eq!(File::from_char('A'), File::A);
        assert_eq!(File::from_char('B'), File::B);
        assert_eq!(File::from_char('C'), File::C);
        assert_eq!(File::from_char('D'), File::D);
        assert_eq!(File::from_char('E'), File::E);
        assert_eq!(File::from_char('F'), File::F);
        assert_eq!(File::from_char('G'), File::G);
        assert_eq!(File::from_char('H'), File::H);
    }

    #[test]
    fn test_square_display() {
        assert_eq!(format!("{}", Square::A1), "A1");
        assert_eq!(format!("{}", Square::H8), "H8");
        assert_eq!(format!("{}", Square::D3), "D3");
        assert_eq!(format!("{}", Square::F6), "F6");
    }
}
