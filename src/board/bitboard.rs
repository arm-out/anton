use super::square::Square;

#[derive(Default)]
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
