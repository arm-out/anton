use bitboard::Bitboard;
use piece::{Color, Piece, PieceType};
use square::Square;
use state::{GameHistory, GameState};
use zobrist::Zobrist;

mod bitboard;
mod fen;
mod piece;
mod square;
mod state;
mod zobrist;

pub struct Board {
    pub bitboards: [[Bitboard; PieceType::COUNT]; Color::COUNT],
    pub occupancy: [Bitboard; Color::COUNT],
    pub mailbox: [Piece; Square::COUNT],
    pub state: GameState,
    pub history: GameHistory,
    pub zobrist: Zobrist,
}

impl Board {
    pub fn new() -> Self {
        const EMPTY: Bitboard = Bitboard(0);
        Self {
            bitboards: [[EMPTY; PieceType::COUNT]; Color::COUNT],
            occupancy: [EMPTY; Color::COUNT],
            mailbox: [Piece::None; Square::COUNT],
            state: GameState {
                active_side: Color::White,
                castling_rights: 0,
                en_passant: Square::None,
                halfmove_clock: 0,
                fullmove_number: 1,
                zobrist_key: 0,
            },
            history: GameHistory::new(),
            zobrist: Zobrist::new(),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self, fen::FenError> {
        let mut board = fen::fen_to_board(Some(fen))?;
        board.state.zobrist_key = Zobrist::init(&board);

        Ok(board)
    }

    pub fn set_piece(&mut self, square: Square, piece: Piece) {
        let color = piece.color();
        let piece_type = piece.piece_type();
        self.bitboards[color][piece_type].set(square);
        self.occupancy[color].set(square);
        self.mailbox[square] = piece;
    }
}
