use crate::board::square::Square;

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)] // Guarantees that the layout of the struct is the same as the underlying type
pub struct Bitboard(pub u64);

impl Bitboard {
    pub fn contains(&self, square: Square) -> bool {
        self.0 & (1 << square as u64) != 0
    }

    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square as u64;
    }

    pub fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square as u64);
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = Square::from_rank_and_file(rank, file);
                if self.contains(square) {
                    write!(f, "X")?;
                } else {
                    write!(f, ".")?;
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
            "XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n\
             XXXXXXXX\n"
        );
        let mut bb = Bitboard(0);
        bb.set(Square::A1);
        bb.set(Square::H8);
        bb.set(Square::D4);
        bb.set(Square::H2);
        assert_eq!(
            format!("{bb}"),
            ".......X\n\
             ........\n\
             ........\n\
             ........\n\
             ...X....\n\
             ........\n\
             .......X\n\
             X.......\n"
        );
    }
}
