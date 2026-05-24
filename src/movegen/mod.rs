use crate::{
    board::{
        Board,
        bitboard::Bitboard,
        castling::CastlingKind,
        piece::{Color, Piece, PieceType},
        square::Square,
    },
    movegen::{
        directions::{KING_SHIFTS, KNIGHT_SHIFTS, MoveShift, PAWN_SHIFT_BLACK, PAWN_SHIFT_WHITE},
        magic::{BISHOP_MAGICS, BISHOP_TABLE_SIZE, Magic, ROOK_MAGICS, ROOK_TABLE_SIZE},
        movelist::MoveList,
        moves::{Move, MoveType, PROMO_CAPTURES, PROMO_TYPES},
    },
};

mod directions;
mod magic;
mod movelist;
pub mod moves;
mod perft;

#[derive(Debug)]
pub struct MoveGenerator {
    king_moves: [Bitboard; Square::COUNT],
    knight_moves: [Bitboard; Square::COUNT],
    pawn_attacks: [[Bitboard; Square::COUNT]; Color::COUNT],
    rook_moves: Vec<Bitboard>,
    bishop_moves: Vec<Bitboard>,
    rook_magics: [Magic; Square::COUNT],
    bishop_magics: [Magic; Square::COUNT],
}

#[derive(Copy, Clone)]
pub enum Slider {
    Rook,
    Bishop,
}

pub const ROOK_DIRS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
pub const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

impl MoveGenerator {
    pub fn new() -> Self {
        let mut movegen = Self {
            king_moves: [Bitboard(0); Square::COUNT],
            knight_moves: [Bitboard(0); Square::COUNT],
            pawn_attacks: [[Bitboard(0); Square::COUNT]; Color::COUNT],
            rook_moves: vec![Bitboard(0); ROOK_TABLE_SIZE],
            bishop_moves: vec![Bitboard(0); BISHOP_TABLE_SIZE],
            rook_magics: ROOK_MAGICS,
            bishop_magics: BISHOP_MAGICS,
        };

        for square in 0..Square::COUNT {
            movegen.init_pawn_attacks(Square::from_idx(square), Color::White);
            movegen.init_pawn_attacks(Square::from_idx(square), Color::Black);
            movegen.init_king_moves(Square::from_idx(square));
            movegen.init_knight_moves(Square::from_idx(square));
        }

        movegen.init_slider_moves(Slider::Rook);
        movegen.init_slider_moves(Slider::Bishop);

        movegen
    }

    pub fn gen_moves(&self, board: &Board) -> MoveList {
        let mask = !board.our_pieces(); // Squares we can move to
        let mut ml = MoveList::default();
        let color = board.us();

        self.gen_king_moves(board, &mut ml, mask, color);
        self.gen_castling_moves(board, &mut ml, color);
        self.gen_knight_moves(board, &mut ml, mask, color);
        self.gen_bishop_moves(board, &mut ml, mask, color);
        self.gen_rook_moves(board, &mut ml, mask, color);
        self.gen_queen_moves(board, &mut ml, mask, color);

        self.gen_pawn_pushes(board, &mut ml, color);
        self.gen_pawn_captures(board, &mut ml, color);

        ml
    }

    // ---------------------- MOVE GENERATION ---------------------

    fn gen_king_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::King, color);

        for square in bb_piece {
            let bb_moves = self.king_moves[square] & mask;
            self.add_moves(board, ml, square, bb_moves, MoveType::Quiet);
        }
    }

    fn gen_castling_moves(&self, board: &Board, ml: &mut MoveList, color: Color) {
        let king_from = match color {
            Color::White => Square::E1,
            Color::Black => Square::E8,
        };
        let king = match color {
            Color::White => Piece::WhiteKing,
            Color::Black => Piece::BlackKing,
        };

        if board.get_piece_at(king_from) != king || self.is_attacked(board, king_from, !color) {
            return;
        }

        let [kingside, queenside] = CastlingKind::KIND_BY_COLOR[color];
        if self.can_castle(board, color, kingside) {
            ml.push(Move::new(
                king_from,
                kingside.castling_destination(),
                MoveType::CastleKingside,
            ));
        }
        if self.can_castle(board, color, queenside) {
            ml.push(Move::new(
                king_from,
                queenside.castling_destination(),
                MoveType::CastleQueenside,
            ));
        }
    }

    fn can_castle(&self, board: &Board, color: Color, kind: CastlingKind) -> bool {
        if !board.state.castling_rights.is_allowed(kind) {
            return false;
        }

        let (rook_square, rook, empty_squares, safe_squares) = match kind {
            CastlingKind::WhiteKingside => (
                Square::H1,
                Piece::WhiteRook,
                [Square::F1, Square::G1, Square::None],
                [Square::F1, Square::G1],
            ),
            CastlingKind::WhiteQueenside => (
                Square::A1,
                Piece::WhiteRook,
                [Square::B1, Square::C1, Square::D1],
                [Square::D1, Square::C1],
            ),
            CastlingKind::BlackKingside => (
                Square::H8,
                Piece::BlackRook,
                [Square::F8, Square::G8, Square::None],
                [Square::F8, Square::G8],
            ),
            CastlingKind::BlackQueenside => (
                Square::A8,
                Piece::BlackRook,
                [Square::B8, Square::C8, Square::D8],
                [Square::D8, Square::C8],
            ),
        };

        if board.get_piece_at(rook_square) != rook {
            return false;
        }

        for square in empty_squares {
            if square != Square::None && board.get_piece_at(square) != Piece::None {
                return false;
            }
        }

        for square in safe_squares {
            if self.is_attacked(board, square, !color) {
                return false;
            }
        }

        true
    }

    fn gen_knight_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Knight, color);
        for square in bb_piece {
            let bb_moves = self.knight_moves[square] & mask;
            self.add_moves(board, ml, square, bb_moves, MoveType::Quiet);
        }
    }

    fn gen_rook_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Rook, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bb_moves = self.get_rook_attacks(square, bb_blockers) & mask;
            self.add_moves(board, ml, square, bb_moves, MoveType::Quiet);
        }
    }

    fn gen_bishop_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Bishop, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bishop_attacks = self.get_bishop_attacks(square, bb_blockers);
            let bb_moves = bishop_attacks & mask;
            self.add_moves(board, ml, square, bb_moves, MoveType::Quiet);
        }
    }

    fn gen_queen_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Queen, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bb_moves = self.get_queen_attacks(square, bb_blockers) & mask;
            self.add_moves(board, ml, square, bb_moves, MoveType::Quiet);
        }
    }

    fn gen_pawn_pushes(&self, board: &Board, ml: &mut MoveList, color: Color) {
        let bb_empty = !board.get_occupancy();
        let bb_promo_rank = Bitboard::promotion_rank(color);
        let bb_fourth_rank = Bitboard::fourth_rank(color);
        let dir: i8 = if color == Color::White { 8 } else { -8 };
        let bb_pieces = board.get_piece(PieceType::Pawn, color);

        for square in bb_pieces {
            // Single push
            let push_square = if dir < 0 {
                square - (dir.abs() as u8)
            } else {
                square + dir as u8
            };
            let one_step = Bitboard::from_square(push_square) & bb_empty;
            let two_steps = if dir > 0 {
                (one_step << dir) & bb_empty & bb_fourth_rank
            } else {
                (one_step >> -dir) & bb_empty & bb_fourth_rank
            };

            let push = one_step & !bb_promo_rank;
            let double_push = two_steps;
            let promos = one_step & bb_promo_rank;

            self.add_moves(board, ml, square, push, MoveType::Quiet);
            self.add_moves(board, ml, square, double_push, MoveType::DoublePawnPush);
            self.add_promotion(ml, square, promos, false);
        }
    }

    fn gen_pawn_captures(&self, board: &Board, ml: &mut MoveList, color: Color) {
        let bb_opponent = board.their_pieces();
        let bb_promo_rank = Bitboard::promotion_rank(color);
        let ep_square = board.get_ep_square();
        let bb_ep_square = match ep_square {
            Square::None => Bitboard(0),
            _ => Bitboard::from_square(ep_square),
        };
        let bb_pieces = board.get_piece(PieceType::Pawn, color);

        for square in bb_pieces {
            let moves = self.pawn_attacks[color][square];
            let captures = moves & bb_opponent & !bb_promo_rank;
            let ep_captures = moves & bb_ep_square;
            let promo_captures = moves & bb_opponent & bb_promo_rank;

            self.add_moves(board, ml, square, captures, MoveType::Quiet);
            self.add_moves(board, ml, square, ep_captures, MoveType::EnPassant);
            self.add_promotion(ml, square, promo_captures, true);
        }
    }

    fn add_promotion(&self, ml: &mut MoveList, from: Square, to: Bitboard, capture: bool) {
        if to.is_empty() {
            return;
        }

        let promo_type = if capture { PROMO_CAPTURES } else { PROMO_TYPES };

        for promo in promo_type {
            for square in to {
                let m = Move::new(from, square, promo);
                ml.push(m);
            }
        }
    }

    fn add_moves(
        &self,
        board: &Board,
        ml: &mut MoveList,
        from: Square,
        to: Bitboard,
        kind: MoveType,
    ) {
        // All genereic moves
        if kind == MoveType::Quiet {
            for square in to {
                let mut flags: u8 = kind as u8;
                // Set capture flag
                if board.mailbox[square] != Piece::None {
                    flags |= 0b0100;
                }

                let m = Move::new(from, square, MoveType::from(flags));
                ml.push(m);
            }
        } else {
            if to == Bitboard(0) {
                return;
            }
            let square = Bitboard::square_from_bb(to);
            let m = Move::new(from, square, kind);
            ml.push(m);
        }
    }

    // ----------------------- INIT HELPERS -----------------------

    fn init_pawn_attacks(&mut self, square: Square, color: Color) {
        let mask = Bitboard::from_square(square);
        let shifts = match color {
            Color::White => PAWN_SHIFT_WHITE,
            Color::Black => PAWN_SHIFT_BLACK,
        };

        for MoveShift { shift, exclude } in shifts {
            let candidate = if color == Color::White {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };

            self.pawn_attacks[color][square] |= candidate;
        }
    }

    fn init_king_moves(&mut self, square: Square) {
        let mask = Bitboard::from_square(square);

        for MoveShift { shift, exclude } in KING_SHIFTS {
            let candidate = if shift > 0 {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };
            self.king_moves[square] |= candidate;
        }
    }

    fn init_knight_moves(&mut self, square: Square) {
        let mask = Bitboard::from_square(square);

        for MoveShift { shift, exclude } in KNIGHT_SHIFTS {
            let candidate = if shift > 0 {
                (mask & !exclude) << shift
            } else {
                (mask & !exclude) >> -shift
            };
            self.knight_moves[square] |= candidate;
        }
    }

    fn init_slider_moves(&mut self, slider: Slider) {
        for square in 0..Square::COUNT {
            let magic = match slider {
                Slider::Rook => self.rook_magics[square],
                Slider::Bishop => self.bishop_magics[square],
            };

            let blockers = MoveGenerator::blocker_boards(magic.mask);
            for blocker in blockers {
                let moves = MoveGenerator::slider_moves(Square::from_idx(square), blocker, slider);
                let table_index = magic.offset + MoveGenerator::magic_index(blocker, &magic);
                let table_entry = match slider {
                    Slider::Rook => &mut self.rook_moves[table_index],
                    Slider::Bishop => &mut self.bishop_moves[table_index],
                };
                if table_entry.is_empty() {
                    *table_entry = moves;
                } else if *table_entry != moves {
                    panic!(
                        "Collision occurred for square {} with blockers {}",
                        square, blocker
                    );
                }
            }
        }
    }

    // --------------------- MAGIC HELPERS ---------------------

    pub fn rook_mask(square: Square) -> Bitboard {
        let mut mask = Bitboard(0);
        let file = square.file();
        let rank = square.rank();
        mask.set_file(file);
        mask.set_rank(rank);
        mask.clear(square);
        mask &= !mask.edges_excluding_square(square);

        mask
    }

    pub fn bishop_mask(square: Square) -> Bitboard {
        let mut mask = Bitboard(0);

        for (df, dr) in BISHOP_DIRS {
            let mut ray = square;
            while let Some(sq) = ray.try_offset(df, dr) {
                mask.set(sq);
                ray = sq;
            }
        }

        mask.clear(square);
        mask &= !mask.edges_excluding_square(square);

        mask
    }

    pub fn blocker_boards(mask: Bitboard) -> Vec<Bitboard> {
        let mut blockers = Vec::new();
        let mut n = 0u64;
        let d = mask.0;

        // Generate all subsets of a set
        // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set
        loop {
            blockers.push(Bitboard(n));
            n = (n.wrapping_sub(d)) & d;
            if n == 0 {
                break;
            }
        }

        blockers
    }

    pub fn slider_moves(square: Square, blockers: Bitboard, slider: Slider) -> Bitboard {
        let mut moves = Bitboard(0);
        let dirs = match slider {
            Slider::Rook => ROOK_DIRS,
            Slider::Bishop => BISHOP_DIRS,
        };

        for (df, dr) in dirs {
            let mut ray = square;
            while !blockers.contains(ray) {
                ray = match ray.try_offset(df, dr) {
                    Some(sq) => sq,
                    None => break,
                };
                moves.set(ray);
            }
        }

        moves
    }

    pub fn magic_index(occupancy: Bitboard, magic: &Magic) -> usize {
        let blockers = occupancy & magic.mask;
        (blockers.0.wrapping_mul(magic.magic) >> magic.shift) as usize
    }

    fn get_rook_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let magic = &self.rook_magics[square];
        let idx = magic.offset + MoveGenerator::magic_index(blockers, magic);
        self.rook_moves[idx]
    }

    fn get_bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let magic = &self.bishop_magics[square];
        let idx = magic.offset + MoveGenerator::magic_index(blockers, magic);
        self.bishop_moves[idx]
    }

    fn get_queen_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let r_magic = &self.rook_magics[square];
        let b_magic = &self.bishop_magics[square];
        let r_idx = r_magic.offset + MoveGenerator::magic_index(blockers, r_magic);
        let b_idx = b_magic.offset + MoveGenerator::magic_index(blockers, b_magic);
        self.rook_moves[r_idx] ^ self.bishop_moves[b_idx]
    }

    // Check if a square is attacked
    // Use superpiece method with early returns
    // https://www.chessprogramming.org/Square_Attacked_By#Any_Attack_by_Side
    pub fn is_attacked(&self, board: &Board, square: Square, color: Color) -> bool {
        let attackers = board.bitboards[color];
        let occupancy = board.get_occupancy();

        let rooks_queen = attackers[PieceType::Rook] | attackers[PieceType::Queen];
        let rook_attacks = self.get_rook_attacks(square, occupancy);
        if rooks_queen & rook_attacks != Bitboard(0) {
            return true;
        }

        let bishops_queen = attackers[PieceType::Bishop] | attackers[PieceType::Queen];
        let bishop_attacks = self.get_bishop_attacks(square, occupancy);
        if bishops_queen & bishop_attacks != Bitboard(0) {
            return true;
        }

        if self.knight_moves[square] & attackers[PieceType::Knight] != Bitboard(0) {
            return true;
        }

        if self.pawn_attacks[!color][square] & attackers[PieceType::Pawn] != Bitboard(0) {
            return true;
        }

        if self.king_moves[square] & attackers[PieceType::King] != Bitboard(0) {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startpos_sliders_are_blocked() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let blockers = board.get_occupancy();

        assert_eq!(
            movegen.get_bishop_attacks(Square::C1, blockers),
            Bitboard::from_square(Square::B2) | Bitboard::from_square(Square::D2)
        );
        assert_eq!(
            movegen.get_rook_attacks(Square::A1, blockers),
            Bitboard::from_square(Square::A2) | Bitboard::from_square(Square::B1)
        );
        assert_eq!(
            movegen.get_queen_attacks(Square::D1, blockers),
            Bitboard::from_square(Square::C1)
                | Bitboard::from_square(Square::D2)
                | Bitboard::from_square(Square::E1)
                | Bitboard::from_square(Square::C2)
                | Bitboard::from_square(Square::E2)
        );
    }

    #[test]
    fn test_black_pawn_double_push_does_not_shift_by_negative_amount() {
        let board =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/N7/PPPPPPPP/R1BQKBNR b KQkq - 1 1").unwrap();
        let movegen = MoveGenerator::new();

        assert_eq!(movegen.gen_moves(&board).len(), 20);
    }

    #[test]
    fn test_pawn_attacks_are_detected_from_target_square() {
        let movegen = MoveGenerator::new();
        let white_attacks = Board::from_fen("8/8/8/3k4/4P3/8/8/7K b - - 0 1").unwrap();
        let black_attacks = Board::from_fen("k7/8/8/3p4/4K3/8/8/8 w - - 0 1").unwrap();

        assert!(movegen.is_attacked(&white_attacks, Square::D5, Color::White));
        assert!(movegen.is_attacked(&black_attacks, Square::E4, Color::Black));
    }

    #[test]
    fn test_quiet_promotion_unmake_ignores_previous_capture() {
        let mut board = Board::from_fen("8/Pk6/8/8/8/8/6Kp/8 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();

        assert!(board.make(
            Move::new(Square::G2, Square::H2, MoveType::Capture),
            &movegen
        ));
        board.unmake();
        assert!(board.make(
            Move::new(Square::A7, Square::A8, MoveType::QPromotion),
            &movegen
        ));
        board.unmake();

        assert_eq!(board.get_piece_at(Square::A7), Piece::WhitePawn);
        assert_eq!(board.get_piece_at(Square::A8), Piece::None);
        assert_eq!(board.get_piece_at(Square::H2), Piece::BlackPawn);
    }

    #[test]
    fn test_castling_moves_are_generated_when_path_is_clear() {
        let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let moves = movegen.gen_moves(&board);
        let mut kingside = false;
        let mut queenside = false;

        for i in 0..moves.len() {
            let m = moves.get(i);
            kingside |= m.from() == Square::E1
                && m.to() == Square::G1
                && m.kind() == MoveType::CastleKingside;
            queenside |= m.from() == Square::E1
                && m.to() == Square::C1
                && m.kind() == MoveType::CastleQueenside;
        }

        assert!(kingside);
        assert!(queenside);
    }

    #[test]
    fn test_castling_moves_rook_and_clears_rights() {
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let castle = Move::new(Square::E1, Square::G1, MoveType::CastleKingside);

        assert!(board.make(castle, &movegen));
        assert_eq!(board.get_piece_at(Square::G1), Piece::WhiteKing);
        assert_eq!(board.get_piece_at(Square::F1), Piece::WhiteRook);
        assert_eq!(board.get_piece_at(Square::E1), Piece::None);
        assert_eq!(board.get_piece_at(Square::H1), Piece::None);
        assert_eq!(board.state.castling_rights.raw() & 0b1100, 0);

        board.unmake();
        assert_eq!(board.get_piece_at(Square::E1), Piece::WhiteKing);
        assert_eq!(board.get_piece_at(Square::H1), Piece::WhiteRook);
        assert_eq!(board.state.castling_rights.raw(), 0b1111);
    }

    #[test]
    fn test_rook_mask() {
        for s in 0..Square::COUNT {
            let square = Square::from_idx(s);
            let mask = MoveGenerator::rook_mask(square);
            println!("Rook mask for square {}: \n{}", square, mask);
        }
    }

    #[test]
    fn test_rook_moves() {
        let square = Square::A1;
        let blockers = Bitboard::from_square(Square::A4);
        let moves = MoveGenerator::slider_moves(square, blockers, Slider::Rook);
        println!(
            "Rook moves for square {} with blockers {}: \n{}",
            square, blockers, moves
        );

        let square = Square::D4;
        let mut blockers = Bitboard::from_square(Square::D7);
        blockers.set(Square::D3);
        blockers.set(Square::D7);
        blockers.set(Square::H4);
        let moves = MoveGenerator::slider_moves(square, blockers, Slider::Rook);
        println!(
            "Rook moves for square {} with blockers {}: \n{}",
            square, blockers, moves
        );
    }
}
