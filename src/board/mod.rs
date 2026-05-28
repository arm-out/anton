use crate::board::castling::{CastlingKind, CastlingPerms};
use crate::evaluation::{Evaluation, phase_value};
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

#[derive(Clone)]
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
                game_phase: 0,
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
        self.add_piece_to_board(square, piece);
        self.state.evaluation.add_piece(square, piece);
        self.state.game_phase += phase_value(piece);
        self.update_hash(square, piece);
    }

    // Remove a piece from the board at the given square
    pub fn remove_piece(&mut self, square: Square, piece: Piece) {
        self.remove_piece_from_board(square, piece);
        self.state.evaluation.remove_piece(square, piece);
        self.state.game_phase -= phase_value(piece);
        self.update_hash(square, piece);
    }

    pub fn move_piece(&mut self, from: Square, to: Square, piece: Piece) {
        self.remove_piece(from, piece);
        self.add_piece(to, piece);
    }

    fn add_piece_to_board(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece.color()][piece.ptype()].set(square);
        self.occupancy[piece.color()].set(square);
        self.mailbox[square] = piece;
    }

    fn remove_piece_from_board(&mut self, square: Square, piece: Piece) {
        self.bitboards[piece.color()][piece.ptype()].clear(square);
        self.occupancy[piece.color()].clear(square);
        self.mailbox[square] = Piece::None;
    }

    fn move_piece_on_board(&mut self, from: Square, to: Square, piece: Piece) {
        self.remove_piece_from_board(from, piece);
        self.add_piece_to_board(to, piece);
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

    pub fn is_repetition(&self) -> bool {
        let current_key = self.state.zobrist_key;
        let reversible_plies = self.state.halfmove_clock as usize;

        self.history
            .iter()
            .rev()
            .take(reversible_plies)
            .skip(1)
            .step_by(2)
            .any(|state| state.zobrist_key == current_key)
    }

    pub fn has_bishop_pair(&self, color: Color) -> bool {
        let bishops = self.get_piece(PieceType::Bishop, color);

        let mut white = 0;
        let mut black = 0;

        if bishops.count_ones() >= 2 {
            for square in bishops {
                match square.color() {
                    Color::White => white += 1,
                    Color::Black => black += 1,
                }
            }
        }

        (white >= 1) && (black >= 1)
    }

    pub fn can_force_checkmate(&self) -> bool {
        let white = self.bitboards[Color::White];
        let black = self.bitboards[Color::Black];

        // Minimum material for checkmate
        white[PieceType::Queen] > Bitboard(0)   // King + 1 Major Piece
            || white[PieceType::Rook] > Bitboard(0)   // King + 1 Major Piece
            || black[PieceType::Queen] > Bitboard(0)   // King + 1 Major Piece
            || black[PieceType::Rook] > Bitboard(0)   // King + 1 Major Piece
            || (white[PieceType::Bishop] > Bitboard(0) && white[PieceType::Knight] > Bitboard(0))   // Bishop + Knight
            || (black[PieceType::Bishop] > Bitboard(0) && black[PieceType::Knight] > Bitboard(0))   // Bishop + Knight
            || white[PieceType::Pawn] > Bitboard(0)   // King + 1 Major Piece (Promo)
            || black[PieceType::Pawn] > Bitboard(0)   // King + 1 Major Piece (Promo)
            || self.has_bishop_pair(Color::White)   // Bishop Pair
            || self.has_bishop_pair(Color::Black) // Bishop Pair
    }

    pub fn draw_by_fifty_rule(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    pub fn is_draw(&self) -> bool {
        (!self.can_force_checkmate()) || self.is_repetition() || self.draw_by_fifty_rule()
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

        debug_assert!(self.check_incrementals());

        return true;
    }

    pub fn unmake(&mut self) {
        let captured = self.state.captured; // Captured Piece

        if let Some(state) = self.history.pop() {
            self.state = state;
        } else {
            return;
        }

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

        debug_assert!(self.check_incrementals());
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
        self.move_piece_on_board(from, to, piece);
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
        self.move_piece_on_board(from, to, piece);
        self.add_piece_to_board(from, captured);
    }

    pub fn unmake_ep(&mut self, m: Move) {
        let from = m.to();
        let to = m.from();
        let us = self.us();
        let (piece, captured) = match us {
            Color::White => (Piece::WhitePawn, Piece::BlackPawn),
            Color::Black => (Piece::BlackPawn, Piece::WhitePawn),
        };

        self.move_piece_on_board(from, to, piece);

        let captured_idx = match us {
            Color::White => from - 8,
            Color::Black => from + 8,
        };

        self.add_piece_to_board(captured_idx, captured);
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
        self.move_piece_on_board(rook_from, rook_to, rook);
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
        self.remove_piece_from_board(from, promo_piece);
        self.add_piece_to_board(from, piece);

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

    fn zobrist_key_from_scratch(&self) -> u64 {
        let mut key = 0;

        for (idx, piece) in self.mailbox.iter().enumerate() {
            if *piece != Piece::None {
                key ^= self.zobrist.pieces[*piece][idx];
            }
        }

        key ^= self.zobrist.castling[self.state.castling_rights];

        if self.state.en_passant != Square::None {
            key ^= self.zobrist.en_passant[self.state.en_passant];
        }

        key ^ self.zobrist.side_to_move[self.state.active_side]
    }

    fn game_phase_from_scratch(&self) -> u8 {
        self.mailbox.iter().map(|piece| phase_value(*piece)).sum()
    }

    // Debug function adapted from Rustic's check_incrementals.
    // https://codeberg.org/mvanthoor/rustic/src/branch/master/rustic/src/board/playmove.rs
    fn check_incrementals(&self) -> bool {
        const CHECK_INCREMENTALS: &str = "Check Incrementals";
        let from_scratch_key = self.zobrist_key_from_scratch();
        let from_scratch_phase = self.game_phase_from_scratch();
        let from_scratch_evaluation = Evaluation::new(self);
        let mut result = true;

        if result && from_scratch_key != self.state.zobrist_key {
            eprintln!(
                "{CHECK_INCREMENTALS}: Error in Zobrist key. incremental={} from_scratch={}",
                self.state.zobrist_key, from_scratch_key
            );
            result = false;
        };

        if result && from_scratch_phase != self.state.game_phase {
            eprintln!(
                "{CHECK_INCREMENTALS}: Error in game phase. incremental={} from_scratch={}",
                self.state.game_phase, from_scratch_phase
            );
            result = false;
        };

        if result && from_scratch_evaluation != self.state.evaluation {
            eprintln!(
                "{CHECK_INCREMENTALS}: Error in evaluation. incremental={:?} from_scratch={:?}",
                self.state.evaluation, from_scratch_evaluation
            );
            result = false;
        };

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uci_move(board: &mut Board, movegen: &MoveGenerator, uci_move: &str) {
        let moves = movegen.gen_moves(board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if m.to_uci() == uci_move {
                assert!(board.make(m, movegen));
                return;
            }
        }

        panic!("legal move not found: {uci_move}");
    }

    #[test]
    fn detects_repetition_from_zobrist_history() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/8/4K1N1 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();

        assert!(!board.is_repetition());

        make_uci_move(&mut board, &movegen, "g1f3");
        make_uci_move(&mut board, &movegen, "e8d8");
        make_uci_move(&mut board, &movegen, "f3g1");
        make_uci_move(&mut board, &movegen, "d8e8");

        assert!(board.is_repetition());
    }
}
