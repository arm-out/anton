use crate::board::Board;

mod board;

fn main() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = Board::from_fen(fen).unwrap();

    for piece in 0..12 {
        println!("Bitboard for piece {}:", piece);
        println!("{}", board.bitboards[piece]);
    }

    for color in 0..2 {
        println!("Occupancy for color {}:", color);
        println!("{}", board.occupancy[color]);
    }

    println!("Mailbox:");
    println!("{:?}", board.mailbox);

    println!("Game state:");
    println!("Active side: {:?}", board.state.active_side);
    println!("Castling rights: {:b}", board.state.castling_rights);
    println!("En passant: {:?}", board.state.en_passant);
    println!("Halfmove clock: {}", board.state.halfmove_clock);
    println!("Fullmove number: {}", board.state.fullmove_number);
    println!("Zobrist key: {}", board.state.zobrist_key);
}
