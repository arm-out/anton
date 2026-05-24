use crate::{
    board::{
        castling::CastlingPerms,
        piece::{Color, Piece},
        square::Square,
    },
    movegen::moves::Move,
};

#[derive(Copy, Clone)]
pub struct GameState {
    pub active_side: Color,
    pub castling_rights: CastlingPerms,
    pub en_passant: Square,
    pub halfmove_clock: u8,
    pub fullmove_number: u16,
    pub zobrist_key: u64,
    pub material: [u16; Color::COUNT],
    pub captured: Piece,
    pub next_move: Move,
}

pub type GameHistory = Vec<GameState>;
