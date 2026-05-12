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
        board.init_hash();

        Ok(board)
    }

    pub fn set_piece(&mut self, square: Square, piece: Piece) {
        let color = piece.color();
        let piece_type = piece.piece_type();
        self.bitboards[color][piece_type].set(square);
        self.occupancy[color].set(square);
        self.mailbox[square] = piece;
    }

    fn update_hash(&mut self, square: Square, piece: Piece) {
        let color = piece.color();
        let piece_type = piece.piece_type();
        self.state.zobrist_key ^= self.zobrist.pieces[color][piece_type][square];
    }

    fn init_hash(&mut self) {
        self.state.zobrist_key = 0;
        for piece_type in 0..PieceType::COUNT {
            for color in 0..Color::COUNT {
                for square in self.bitboards[color][piece_type] {
                    self.state.zobrist_key ^= self.zobrist.pieces[color][piece_type][square];
                }
            }
        }

        self.state.zobrist_key ^= self.zobrist.castling[self.state.castling_rights as usize]; // Castling rights
        if self.state.en_passant != Square::None {
            self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // En passant
        }
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Side to move
    }
}
