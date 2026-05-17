use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Not, Shl, Shr};

use crate::board::{
    piece::Color,
    square::{File, Rank, Square},
};

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)] // Guarantees that the layout of the struct is the same as the underlying type
pub struct Bitboard(pub u64);

impl Bitboard {
    pub fn contains(&self, square: Square) -> bool {
        self.0 & (1 << square as u64) != 0
    }

    #[inline]
    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square as u64;
    }

    #[inline]
    pub fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square as u64);
    }

    pub fn set_rank(&mut self, rank: Rank) {
        self.0 |= 0xFF << ((rank as u8) * 8);
    }

    pub fn set_file(&mut self, file: File) {
        self.0 |= 0x0101010101010101 << (file as u8);
    }

    pub fn from_square(square: Square) -> Self {
        Bitboard(1 << square as u8)
    }

    pub fn promotion_rank(color: Color) -> Self {
        match color {
            Color::White => Bitboard::from_rank(Rank::R8),
            Color::Black => Bitboard::from_rank(Rank::R1),
        }
    }

    pub fn fourth_rank(color: Color) -> Self {
        match color {
            Color::White => Bitboard::from_rank(Rank::R4),
            Color::Black => Bitboard::from_rank(Rank::R5),
        }
    }

    pub fn square_from_bb(bb: Bitboard) -> Square {
        unsafe { std::mem::transmute(bb.0.trailing_zeros() as u8) }
    }

    pub const fn from_file(file: File) -> Self {
        match file {
            File::A => Bitboard(0x0101010101010101),
            File::B => Bitboard(0x0202020202020202),
            File::C => Bitboard(0x0404040404040404),
            File::D => Bitboard(0x0808080808080808),
            File::E => Bitboard(0x1010101010101010),
            File::F => Bitboard(0x2020202020202020),
            File::G => Bitboard(0x4040404040404040),
            File::H => Bitboard(0x8080808080808080),
        }
    }

    pub const fn from_rank(rank: Rank) -> Self {
        match rank {
            Rank::R1 => Bitboard(0x00000000000000FF),
            Rank::R2 => Bitboard(0x000000000000FF00),
            Rank::R3 => Bitboard(0x0000000000FF0000),
            Rank::R4 => Bitboard(0x00000000FF000000),
            Rank::R5 => Bitboard(0x000000FF00000000),
            Rank::R6 => Bitboard(0x0000FF0000000000),
            Rank::R7 => Bitboard(0x00FF000000000000),
            Rank::R8 => Bitboard(0xFF00000000000000),
        }
    }

    pub const fn union(self, other: Self) -> Self {
        Bitboard(self.0 | other.0)
    }

    pub fn edges_excluding_square(self, square: Square) -> Bitboard {
        let file = Bitboard::from_file(square.file());
        let rank = Bitboard::from_rank(square.rank());

        (Bitboard::from_file(File::A) & !file)
            | (Bitboard::from_file(File::H) & !file)
            | (Bitboard::from_rank(Rank::R1) & !rank)
            | (Bitboard::from_rank(Rank::R8) & !rank)
    }

    pub fn count_ones(self) -> u8 {
        self.0.count_ones() as u8
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let square_idx = self.0.trailing_zeros() as u8; // Get index of least significant bit
        self.0 &= self.0 - 1;
        Some(unsafe { std::mem::transmute(square_idx) }) // Convert index to Square
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Shl<i8> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: i8) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<i8> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: i8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                if self.contains(square) {
                    write!(f, "X ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bitboard_construction() {
        let bb = Bitboard(0);
        assert_eq!(bb.0, 0x0000000000000000);
    }

    #[test]
    fn test_bitboard_contains() {
        let mut bb = Bitboard(1);
        assert!(bb.contains(Square::A1));
        assert!(!bb.contains(Square::H8));
        bb.set(Square::H8);
        assert!(bb.contains(Square::H8));
        bb.set(Square::D4);
        assert!(bb.contains(Square::D4));
        bb.set(Square::H2);
        assert!(bb.contains(Square::H2));
    }

    #[test]
    fn test_bitboard_set() {
        let mut bb = Bitboard(0);
        bb.set(Square::A1);
        assert!(bb.contains(Square::A1));
        bb.set(Square::H8);
        assert!(bb.contains(Square::H8));
        bb.set(Square::D4);
        assert!(bb.contains(Square::D4));
        bb.set(Square::H2);
        assert!(bb.contains(Square::H2));
    }

    #[test]
    fn test_bitboard_clear() {
        let mut bb = Bitboard(0xFFFFFFFFFFFFFFFF);
        bb.clear(Square::A1);
        assert!(!bb.contains(Square::A1));
        bb.clear(Square::H8);
        assert!(!bb.contains(Square::H8));
        bb.clear(Square::D4);
        assert!(!bb.contains(Square::D4));
        bb.clear(Square::H2);
        assert!(!bb.contains(Square::H2));
    }

    #[test]
    fn test_bitboard_display() {
        let bb = Bitboard(0xFFFFFFFFFFFFFFFF);
        assert_eq!(
            format!("{bb}"),
            "X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n\
             X X X X X X X X \n"
        );
        let mut bb = Bitboard(0);
        bb.set(Square::A1);
        bb.set(Square::H8);
        bb.set(Square::D4);
        bb.set(Square::H2);
        assert_eq!(
            format!("{bb}"),
            ". . . . . . . X \n\
             . . . . . . . . \n\
             . . . . . . . . \n\
             . . . . . . . . \n\
             . . . X . . . . \n\
             . . . . . . . . \n\
             . . . . . . . X \n\
             X . . . . . . . \n"
        );
    }

    #[test]
    fn test_bitboard_iterator() {
        let bb = Bitboard(0x8100_0000_0000_0000);
        for (square, assert_square) in bb.into_iter().zip([Square::A8, Square::H8].iter()) {
            assert_eq!(square, *assert_square);
        }
    }

    #[test]
    fn test_bitboard_set_rank() {
        let mut bb = Bitboard(0);
        bb.set_rank(Rank::R1);
        assert!(bb.contains(Square::A1));
        bb.set_rank(Rank::R8);
        assert!(bb.contains(Square::A8));
        bb.set_rank(Rank::R2);
        assert!(bb.contains(Square::B1));
    }

    #[test]
    fn test_bitboard_set_file() {
        let mut bb = Bitboard(0);
        bb.set_file(File::A);
        assert!(bb.contains(Square::A1));
        bb.set_file(File::H);
        assert!(bb.contains(Square::H1));
        bb.set_file(File::D);
        assert!(bb.contains(Square::D1));
    }

    #[test]
    fn test_bitboard_from_file() {
        let mut a = Bitboard(0);
        a.set_file(File::A);
        assert_eq!(a, Bitboard::from_file(File::A));
        let mut b = Bitboard(0);
        b.set_file(File::B);
        assert_eq!(b, Bitboard::from_file(File::B));
        let mut c = Bitboard(0);
        c.set_file(File::C);
        assert_eq!(c, Bitboard::from_file(File::C));
        let mut d = Bitboard(0);
        d.set_file(File::D);
        assert_eq!(d, Bitboard::from_file(File::D));
        let mut e = Bitboard(0);
        e.set_file(File::E);
        assert_eq!(e, Bitboard::from_file(File::E));
        let mut f = Bitboard(0);
        f.set_file(File::F);
        assert_eq!(f, Bitboard::from_file(File::F));
        let mut g = Bitboard(0);
        g.set_file(File::G);
        assert_eq!(g, Bitboard::from_file(File::G));
        let mut h = Bitboard(0);
        h.set_file(File::H);
        assert_eq!(h, Bitboard::from_file(File::H));
    }

    #[test]
    fn test_bitboard_from_rank() {
        let mut r1 = Bitboard(0);
        r1.set_rank(Rank::R1);
        assert_eq!(r1, Bitboard::from_rank(Rank::R1));
        let mut r2 = Bitboard(0);
        r2.set_rank(Rank::R2);
        assert_eq!(r2, Bitboard::from_rank(Rank::R2));
        let mut r3 = Bitboard(0);
        r3.set_rank(Rank::R3);
        assert_eq!(r3, Bitboard::from_rank(Rank::R3));
        let mut r4 = Bitboard(0);
        r4.set_rank(Rank::R4);
        assert_eq!(r4, Bitboard::from_rank(Rank::R4));
        let mut r5 = Bitboard(0);
        r5.set_rank(Rank::R5);
        assert_eq!(r5, Bitboard::from_rank(Rank::R5));
        let mut r6 = Bitboard(0);
        r6.set_rank(Rank::R6);
        assert_eq!(r6, Bitboard::from_rank(Rank::R6));
        let mut r7 = Bitboard(0);
        r7.set_rank(Rank::R7);
        assert_eq!(r7, Bitboard::from_rank(Rank::R7));
        let mut r8 = Bitboard(0);
        r8.set_rank(Rank::R8);
        assert_eq!(r8, Bitboard::from_rank(Rank::R8));
    }
}
