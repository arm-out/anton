mod board;

fn main() {
    let mut test_bb = board::bitboard::Bitboard(0);
    println!("Empty Bitboard");
    println!("{test_bb}");
    println!("Bitboard with A1 and H8 set");
    test_bb.set(board::square::Square::A1);
    println!("{test_bb}");
    test_bb.set(board::square::Square::H8);
    println!("{test_bb}");
    println!("Bitboard with A1 and H8 cleared");
    test_bb.clear(board::square::Square::A1);
    println!("{test_bb}");
    test_bb.clear(board::square::Square::H8);
    println!("{test_bb}");
    test_bb.set(board::square::Square::A1);
    test_bb.set(board::square::Square::H8);
    test_bb.set(board::square::Square::D4);
    println!("Bitboard with A1, D4 and H8 set");
    println!("{test_bb}");
}
