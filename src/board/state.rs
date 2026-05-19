use crate::{
    board::{piece::Color, square::Square},
    movegen::moves::Move,
};

#[derive(Copy, Clone)]
pub struct GameState {
    pub active_side: Color,
    pub castling_rights: u8, // 4 bits for KQkq
    pub en_passant: Square,
    pub halfmove_clock: u8,
    pub fullmove_number: u16,
    pub zobrist_key: u64,
    // pub phase_value: i16,
    // pub psqt_value: i16,
    pub next_move: Move,
}

pub type GameHistory = Vec<GameState>;
