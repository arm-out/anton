use anton::{
    board::{Board, piece::Color},
    evaluation::evaluate_static,
    movegen::MoveGenerator,
};

fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "white",
        Color::Black => "black",
    }
}

#[test]
#[ignore]
fn print_12move_evaluations() {
    println!(
        "{:<4} {:<5} {:>5} {:>6}  {}",
        "case", "side", "phase", "eval", "fen"
    );

    let movegen = MoveGenerator::new();
    for (idx, fen) in include_str!("12move.epd").lines().enumerate() {
        let board = Board::from_fen(fen)
            .unwrap_or_else(|err| panic!("Invalid FEN on line {}: {err}", idx + 1));
        let eval = evaluate_static(&board, &movegen);

        println!(
            "{:<4} {:<5} {:>5} {:>6}  {}",
            idx + 1,
            color_name(board.state.active_side),
            board.state.game_phase,
            eval,
            fen
        );
    }
}
