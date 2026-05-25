use crate::board::castling::{CastlingKind, CastlingPerms};
use crate::evaluation::Evaluation;
use crate::movegen::MoveGenerator;
use crate::movegen::moves::{Move, MoveType};
use bitboard::Bitboard;
use piece::PieceType;
use piece::{Color, Piece};
use square::Square;
use state::{GameHistory, GameState};
use zobrist::Zobrist;

pub mod bitboard;
pub mod castling;
mod fen;
pub mod piece;
pub mod square;
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
                castling_rights: CastlingPerms { raw: 0b0000 },
                en_passant: Square::None,
                halfmove_clock: 0,
                fullmove_number: 1,
                zobrist_key: 0,
                evaluation: Evaluation::default(),
                next_move: Move(0),
                captured: Piece::None,
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

    // --------------------- MOVEMENT HELPERS ---------------------

    // Add a piece to the board at the given square
    pub fn add_piece(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece.color()][piece.ptype()].set(square);
        self.occupancy[piece.color()].set(square);
        self.mailbox[square] = piece;
        self.state.evaluation.add_piece(piece);
        self.update_hash(square, piece);
    }

    // Remove a piece from the board at the given square
    pub fn remove_piece(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece.color()][piece.ptype()].clear(square);
        self.occupancy[piece.color()].clear(square);
        self.mailbox[square] = Piece::None;
        self.state.evaluation.remove_piece(piece);
        self.update_hash(square, piece);
    }

    pub fn move_piece(&mut self, from: Square, to: Square, piece: Piece) {
        self.remove_piece(from, piece);
        self.add_piece(to, piece);
    }

    // Side to move
    pub fn us(&self) -> Color {
        self.state.active_side
    }

    // Oponent Side
    pub fn them(&self) -> Color {
        !self.state.active_side
    }

    // Our Pieces
    pub fn our_pieces(&self) -> Bitboard {
        self.occupancy[self.state.active_side]
    }

    pub fn their_pieces(&self) -> Bitboard {
        self.occupancy[self.them()]
    }

    pub fn get_occupancy(&self) -> Bitboard {
        self.occupancy[self.us()] | self.occupancy[self.them()]
    }

    pub fn get_piece(&self, piece_type: PieceType, color: Color) -> Bitboard {
        self.bitboards[color][piece_type]
    }

    pub fn get_ep_square(&self) -> Square {
        self.state.en_passant
    }

    pub fn get_piece_at(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    // Make Move
    // 1. Push current state to history
    // 2. Update State and Board
    //    - Update Bitboards and Mailbox
    //    - Update halfmove
    //    - Update EP Squares
    //    - Update Castling Rights
    //    - Update Full Move Number
    //    - Update Zobrist Key
    // 3. Check if legal
    pub fn make(&mut self, m: Move, mg: &MoveGenerator) -> bool {
        // Push current state to history
        let mut curr_state = self.state;
        curr_state.next_move = m;
        self.history.push(curr_state);

        let move_kind = m.kind();
        let us: Color = self.us();
        let moving_piece = self.get_piece_at(m.from());
        let captured_piece = match move_kind {
            MoveType::Capture
            | MoveType::NPromoCapture
            | MoveType::BPromoCapture
            | MoveType::RPromoCapture
            | MoveType::QPromoCapture => self.get_piece_at(m.to()),
            MoveType::EnPassant => match us {
                Color::White => Piece::BlackPawn,
                Color::Black => Piece::WhitePawn,
            },
            _ => Piece::None,
        };

        self.clear_ep_square();
        self.state.halfmove_clock += 1;
        self.state.captured = Piece::None;

        match move_kind {
            MoveType::Quiet => self.make_quiet(m),
            MoveType::DoublePawnPush => {
                self.make_quiet(m);
                self.set_ep_square(Square::ep_square(m.from()));
            }
            MoveType::Capture => self.make_capture(m),
            MoveType::EnPassant => self.make_ep(m, us),
            MoveType::NPromotion => self.make_promotion(m, PieceType::Knight, false),
            MoveType::BPromotion => self.make_promotion(m, PieceType::Bishop, false),
            MoveType::RPromotion => self.make_promotion(m, PieceType::Rook, false),
            MoveType::QPromotion => self.make_promotion(m, PieceType::Queen, false),
            MoveType::NPromoCapture => self.make_promotion(m, PieceType::Knight, true),
            MoveType::BPromoCapture => self.make_promotion(m, PieceType::Bishop, true),
            MoveType::RPromoCapture => self.make_promotion(m, PieceType::Rook, true),
            MoveType::QPromoCapture => self.make_promotion(m, PieceType::Queen, true),
            MoveType::CastleKingside => self.make_castle(m, true),
            MoveType::CastleQueenside => self.make_castle(m, false),
        }

        self.update_castling_rights_for_move(m, moving_piece, captured_piece);

        // Update full move number if we are black
        if us == Color::Black {
            self.state.fullmove_number += 1;
        }

        self.toggle_side();

        // Check legality
        let king = self.bitboards[us][PieceType::King];
        let king_square = Square::from_idx(king.0.trailing_zeros() as usize);
        if mg.is_attacked(self, king_square, self.us()) {
            self.unmake();
            return false;
        }

        return true;
    }

    pub fn unmake(&mut self) {
        let captured = self.state.captured; // Captured Piece
        let evaluation = self.state.evaluation;

        if let Some(state) = self.history.pop() {
            self.state = state;
        } else {
            return;
        }

        let restored_evaluation = self.state.evaluation;
        self.state.evaluation = evaluation;

        // Move to undo
        let m = self.state.next_move;
        let move_kind = m.kind();

        match move_kind {
            MoveType::Quiet => self.unmake_quiet(m),
            MoveType::DoublePawnPush => self.unmake_quiet(m),
            MoveType::Capture => self.unmake_capture(m, captured),
            MoveType::EnPassant => self.unmake_ep(m),
            MoveType::NPromotion => self.unmake_promotion(m, PieceType::Knight, captured),
            MoveType::BPromotion => self.unmake_promotion(m, PieceType::Bishop, captured),
            MoveType::RPromotion => self.unmake_promotion(m, PieceType::Rook, captured),
            MoveType::QPromotion => self.unmake_promotion(m, PieceType::Queen, captured),
            MoveType::NPromoCapture => self.unmake_promotion(m, PieceType::Knight, captured),
            MoveType::BPromoCapture => self.unmake_promotion(m, PieceType::Bishop, captured),
            MoveType::RPromoCapture => self.unmake_promotion(m, PieceType::Rook, captured),
            MoveType::QPromoCapture => self.unmake_promotion(m, PieceType::Queen, captured),
            MoveType::CastleKingside => self.unmake_castle(m, true),
            MoveType::CastleQueenside => self.unmake_castle(m, false),
        }

        debug_assert_eq!(self.state.evaluation, restored_evaluation);
    }

    pub fn make_quiet(&mut self, m: Move) {
        let from = m.from();
        let to = m.to();
        let piece = self.get_piece_at(from);
        self.move_piece(from, to, piece);

        if matches!(piece, Piece::WhitePawn | Piece::BlackPawn) {
            self.state.halfmove_clock = 0;
        }
    }

    pub fn unmake_quiet(&mut self, m: Move) {
        let from = m.to();
        let to = m.from();
        let piece = self.get_piece_at(from);
        self.move_piece(from, to, piece);
    }

    pub fn make_capture(&mut self, m: Move) {
        let from = m.from();
        let to = m.to();
        let captured = self.get_piece_at(to);
        let piece = self.get_piece_at(from);
        self.remove_piece(to, captured);
        self.move_piece(from, to, piece);
        self.state.halfmove_clock = 0;
        self.state.captured = captured;
    }

    pub fn make_ep(&mut self, m: Move, color: Color) {
        let from = m.from();
        let to = m.to();
        let captured_idx = if color == Color::White {
            to - 8
        } else {
            to + 8
        };
        let (piece, captured) = match color {
            Color::White => (Piece::WhitePawn, Piece::BlackPawn),
            Color::Black => (Piece::BlackPawn, Piece::WhitePawn),
        };

        self.remove_piece(captured_idx, captured);
        self.move_piece(from, to, piece);
        self.state.halfmove_clock = 0;
        self.state.captured = captured;
    }

    pub fn unmake_capture(&mut self, m: Move, captured: Piece) {
        let from = m.to();
        let to = m.from();
        let piece = self.get_piece_at(from);
        self.move_piece(from, to, piece);
        self.add_piece(from, captured);
    }

    pub fn unmake_ep(&mut self, m: Move) {
        let from = m.to();
        let to = m.from();
        let us = self.us();
        let (piece, captured) = match us {
            Color::White => (Piece::WhitePawn, Piece::BlackPawn),
            Color::Black => (Piece::BlackPawn, Piece::WhitePawn),
        };

        self.move_piece(from, to, piece);

        let captured_idx = match us {
            Color::White => from - 8,
            Color::Black => from + 8,
        };

        self.add_piece(captured_idx, captured);
    }

    pub fn make_castle(&mut self, m: Move, kingside: bool) {
        self.make_quiet(m);
        let (rook_from, rook_to, rook) = match (self.us(), kingside) {
            (Color::White, true) => (Square::H1, Square::F1, Piece::WhiteRook),
            (Color::White, false) => (Square::A1, Square::D1, Piece::WhiteRook),
            (Color::Black, true) => (Square::H8, Square::F8, Piece::BlackRook),
            (Color::Black, false) => (Square::A8, Square::D8, Piece::BlackRook),
        };
        self.move_piece(rook_from, rook_to, rook);
    }

    pub fn unmake_castle(&mut self, m: Move, kingside: bool) {
        self.unmake_quiet(m);
        let (rook_from, rook_to, rook) = match (self.us(), kingside) {
            (Color::White, true) => (Square::F1, Square::H1, Piece::WhiteRook),
            (Color::White, false) => (Square::D1, Square::A1, Piece::WhiteRook),
            (Color::Black, true) => (Square::F8, Square::H8, Piece::BlackRook),
            (Color::Black, false) => (Square::D8, Square::A8, Piece::BlackRook),
        };
        self.move_piece(rook_from, rook_to, rook);
    }

    pub fn make_promotion(&mut self, m: Move, pt: PieceType, capture: bool) {
        if capture {
            self.make_capture(m);
        } else {
            self.make_quiet(m);
        }

        let to = m.to();
        let promo_piece = Piece::from_index(((2 * pt as u8) + self.us() as u8) as usize);
        self.remove_piece(to, self.get_piece_at(to));
        self.add_piece(to, promo_piece);
    }

    pub fn unmake_promotion(&mut self, m: Move, pt: PieceType, captured: Piece) {
        let from = m.to();
        let us = self.us();
        let piece = match us {
            Color::White => Piece::WhitePawn,
            Color::Black => Piece::BlackPawn,
        };

        let promo_piece = Piece::from_index(((2 * pt as u8) + us as u8) as usize);
        self.remove_piece(from, promo_piece);
        self.add_piece(from, piece);

        if captured != Piece::None {
            self.unmake_capture(m, captured);
        } else {
            self.unmake_quiet(m);
        }
    }

    // ----------------------- STATE HELPERS ------------------------

    pub fn set_ep_square(&mut self, square: Square) {
        // Clear old EP square
        if self.state.en_passant != Square::None {
            self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // Remove old EP square from hash
        }
        if square != Square::None {
            self.state.en_passant = square;
            self.state.zobrist_key ^= self.zobrist.en_passant[square]; // Add new EP square to hash
        }
    }

    pub fn clear_ep_square(&mut self) {
        if self.state.en_passant == Square::None {
            return;
        }

        self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // Remove old EP square from hash
        self.state.en_passant = Square::None;
    }

    pub fn toggle_side(&mut self) {
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Remove old side from hash
        self.state.active_side = !self.state.active_side;
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Add new side to hash
    }

    pub fn update_castling_rights(&mut self, new_rights: CastlingPerms) {
        if self.state.castling_rights.raw == new_rights.raw {
            return;
        }
        self.state.zobrist_key ^= self.zobrist.castling[self.state.castling_rights]; // Remove old castling rights from hash
        self.state.castling_rights = new_rights;
        self.state.zobrist_key ^= self.zobrist.castling[new_rights]; // Add new castling rights to hash
    }

    fn update_castling_rights_for_move(&mut self, m: Move, moving: Piece, captured: Piece) {
        let mut rights = self.state.castling_rights.raw;

        match moving {
            Piece::WhiteKing => {
                rights &= !(CastlingKind::WhiteKingside as u8);
                rights &= !(CastlingKind::WhiteQueenside as u8);
            }
            Piece::BlackKing => {
                rights &= !(CastlingKind::BlackKingside as u8);
                rights &= !(CastlingKind::BlackQueenside as u8);
            }
            Piece::WhiteRook => match m.from() {
                Square::H1 => rights &= !(CastlingKind::WhiteKingside as u8),
                Square::A1 => rights &= !(CastlingKind::WhiteQueenside as u8),
                _ => {}
            },
            Piece::BlackRook => match m.from() {
                Square::H8 => rights &= !(CastlingKind::BlackKingside as u8),
                Square::A8 => rights &= !(CastlingKind::BlackQueenside as u8),
                _ => {}
            },
            _ => {}
        }

        match (captured, m.to()) {
            (Piece::WhiteRook, Square::H1) => rights &= !(CastlingKind::WhiteKingside as u8),
            (Piece::WhiteRook, Square::A1) => rights &= !(CastlingKind::WhiteQueenside as u8),
            (Piece::BlackRook, Square::H8) => rights &= !(CastlingKind::BlackKingside as u8),
            (Piece::BlackRook, Square::A8) => rights &= !(CastlingKind::BlackQueenside as u8),
            _ => {}
        }

        self.update_castling_rights(CastlingPerms { raw: rights });
    }

    // ----------------------- HASH FUNCTIONS -----------------------

    fn update_hash(&mut self, square: Square, piece: Piece) {
        self.state.zobrist_key ^= self.zobrist.pieces[piece][square];
    }

    fn init_hash(&mut self) {
        self.state.zobrist_key = 0;
        for (i, piece) in self.mailbox.iter().enumerate() {
            if *piece == Piece::None {
                continue;
            }
            self.state.zobrist_key ^= self.zobrist.pieces[*piece][i];
        }

        self.state.zobrist_key ^= self.zobrist.castling[self.state.castling_rights]; // Castling rights
        if self.state.en_passant != Square::None {
            self.state.zobrist_key ^= self.zobrist.en_passant[self.state.en_passant]; // En passant
        }
        self.state.zobrist_key ^= self.zobrist.side_to_move[self.state.active_side]; // Side to move
    }
}
