use bitboard::Bitboard;
use piece::{Color, Piece, PieceType};
use square::Square;
use state::{GameHistory, GameState};
use zobrist::Zobrist;

pub mod bitboard;
mod fen;
pub mod piece;
pub mod square;
mod state;
mod zobrist;

pub struct Board {
    pub bitboards: [Bitboard; Piece::COUNT],
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
            bitboards: [EMPTY; Piece::COUNT],
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
        board.init_hash();

        Ok(board)
    }

    // --------------------- MOVEMENT FUNCTIONS ---------------------

    pub fn move_piece(&mut self, from: Square, to: Square, piece: Piece) {
        self.remove_piece(from, piece);
        self.add_piece(to, piece);
    }

    // Add a piece to the board at the given square
    pub fn add_piece(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece].set(square);
        self.occupancy[piece.color()].set(square);
        self.mailbox[square] = piece;
    }

    // Remove a piece from the board at the given square
    pub fn remove_piece(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece].clear(square);
        self.occupancy[piece.color()].clear(square);
        self.mailbox[square] = Piece::None;
    }

    pub fn set_ep_square(&mut self, square: Square) {
        self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // Remove old EP square from hash
        self.state.en_passant = square;
        self.state.zobrist_key ^= self.zobrist.en_passant[square]; // Add new EP square to hash
    }

    pub fn clear_ep_square(&mut self) {
        self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // Remove old EP square from hash
        self.state.en_passant = Square::None;
    }

    pub fn toggle_side(&mut self) {
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Remove old side from hash
        self.state.active_side = !self.state.active_side;
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Add new side to hash
    }

    pub fn update_castling_rights(&mut self, new_rights: u8) {
        self.state.zobrist_key ^= self.zobrist.castling[self.state.castling_rights as usize]; // Remove old castling rights from hash
        self.state.castling_rights = new_rights;
        self.state.zobrist_key ^= self.zobrist.castling[new_rights as usize]; // Add new castling rights to hash
    }

    // --------------------- HASH FUNCTIONS ---------------------

    fn update_hash(&mut self, square: Square, piece: Piece) {
        self.state.zobrist_key ^= self.zobrist.pieces[piece][square];
    }

    fn init_hash(&mut self) {
        self.state.zobrist_key = 0;
        for piece in 0..Piece::COUNT {
            for square in self.bitboards[piece] {
                self.state.zobrist_key ^= self.zobrist.pieces[piece][square];
            }
        }

        self.state.zobrist_key ^= self.zobrist.castling[self.state.castling_rights as usize]; // Castling rights
        if self.state.en_passant != Square::None {
            self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // En passant
        }
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Side to move
    }
}
