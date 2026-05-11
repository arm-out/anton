#[rustfmt::skip]
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
}

impl Square {
    pub const TOTAL: u8 = 64;

    pub fn new(square: u8) -> Self {
        unsafe { std::mem::transmute(square) }
    }

    pub fn from_rank_and_file(rank: u8, file: u8) -> Self {
        Self::new(rank << 3 | file)
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
}
