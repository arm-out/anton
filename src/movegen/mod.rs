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
        moves::{Move, MoveType, PROMO_CAPTURES, PROMO_TYPES},
    },
};

mod directions;
mod magic;
mod movelist;
pub mod moves;

pub(crate) use movelist::{MAX_MOVES, MoveList};

#[derive(Debug)]
pub struct MoveGenerator {
    king_moves: [Bitboard; Square::COUNT],
    knight_moves: [Bitboard; Square::COUNT],
    pawn_attacks: [[Bitboard; Square::COUNT]; Color::COUNT],
    rook_moves: Vec<Bitboard>,
    bishop_moves: Vec<Bitboard>,
    rook_magics: [Magic; Square::COUNT],
    bishop_magics: [Magic; Square::COUNT],
    between: [[Bitboard; Square::COUNT]; Square::COUNT],
}

pub trait GenType {
    const CAPTURES: bool;
    const QUIETS: bool;
    const PROMOTIONS: bool;
    const CASTLING: bool;
    const EVASIONS: bool = false;
}

pub struct All;
pub struct Noisy;
pub struct Quiet;
pub struct Evasions;

impl GenType for All {
    const CAPTURES: bool = true;
    const QUIETS: bool = true;
    const PROMOTIONS: bool = true;
    const CASTLING: bool = true;
}

impl GenType for Noisy {
    const CAPTURES: bool = true;
    const QUIETS: bool = false;
    const PROMOTIONS: bool = true;
    const CASTLING: bool = false;
}

impl GenType for Quiet {
    const CAPTURES: bool = false;
    const QUIETS: bool = true;
    const PROMOTIONS: bool = false;
    const CASTLING: bool = true;
}

impl GenType for Evasions {
    const CAPTURES: bool = true;
    const QUIETS: bool = true;
    const PROMOTIONS: bool = true;
    const CASTLING: bool = false;
    const EVASIONS: bool = true;
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
            between: [[Bitboard(0); Square::COUNT]; Square::COUNT],
        };

        for square in 0..Square::COUNT {
            movegen.init_pawn_attacks(Square::from_idx(square), Color::White);
            movegen.init_pawn_attacks(Square::from_idx(square), Color::Black);
            movegen.init_king_moves(Square::from_idx(square));
            movegen.init_knight_moves(Square::from_idx(square));
        }

        movegen.init_slider_moves(Slider::Rook);
        movegen.init_slider_moves(Slider::Bishop);
        movegen.init_between();

        movegen
    }

    pub fn gen_moves<G: GenType>(&self, board: &Board) -> MoveList {
        if G::EVASIONS {
            return self.gen_evasions(board);
        }

        let mut ml = MoveList::default();
        let color = board.us();
        let targets = Bitboard(u64::MAX);

        self.gen_king_moves::<G>(board, &mut ml, color, targets);
        if G::CASTLING {
            self.gen_castling_moves(board, &mut ml, color);
        }
        self.gen_knight_moves::<G>(board, &mut ml, color, targets);
        self.gen_bishop_moves::<G>(board, &mut ml, color, targets);
        self.gen_rook_moves::<G>(board, &mut ml, color, targets);
        self.gen_queen_moves::<G>(board, &mut ml, color, targets);
        self.gen_pawn_moves::<G>(board, &mut ml, color, targets);

        ml
    }

    pub fn is_pseudolegal_killer(&self, board: &Board, m: Move) -> bool {
        let from = m.from();
        let to = m.to();

        if from == Square::None || to == Square::None {
            return false;
        }

        let piece = board.get_piece_at(from);
        if piece == Piece::None || piece.color() != board.us() {
            return false;
        }

        match m.kind() {
            MoveType::Quiet => {
                board.get_piece_at(to) == Piece::None
                    && self.pseudolegal_quiet_piece_move(board, piece.ptype(), from, to)
            }
            MoveType::DoublePawnPush => {
                piece.ptype() == PieceType::Pawn
                    && board.get_piece_at(to) == Piece::None
                    && self.pseudolegal_double_pawn_push(board, from, to)
            }
            MoveType::CastleKingside | MoveType::CastleQueenside => {
                piece.ptype() == PieceType::King && self.pseudolegal_castle(board, m)
            }
            _ => false,
        }
    }

    fn pseudolegal_quiet_piece_move(
        &self,
        board: &Board,
        piece_type: PieceType,
        from: Square,
        to: Square,
    ) -> bool {
        match piece_type {
            PieceType::Pawn => self.pseudolegal_single_pawn_push(board.us(), from, to)
                && !Bitboard::promotion_rank(board.us()).contains(to),
            PieceType::Knight => self.knight_moves[from].contains(to),
            PieceType::Bishop => self.get_bishop_attacks(from, board.get_occupancy()).contains(to),
            PieceType::Rook => self.get_rook_attacks(from, board.get_occupancy()).contains(to),
            PieceType::Queen => self.get_queen_attacks(from, board.get_occupancy()).contains(to),
            PieceType::King => self.king_moves[from].contains(to),
            PieceType::None => false,
        }
    }

    fn pseudolegal_single_pawn_push(&self, color: Color, from: Square, to: Square) -> bool {
        let step = match color {
            Color::White => from.try_offset(0, 1),
            Color::Black => from.try_offset(0, -1),
        };

        step == Some(to)
    }

    fn pseudolegal_double_pawn_push(&self, board: &Board, from: Square, to: Square) -> bool {
        let (one_step, start_rank) = match board.us() {
            Color::White => (from.try_offset(0, 1), 1),
            Color::Black => (from.try_offset(0, -1), 6),
        };

        let two_step = match board.us() {
            Color::White => from.try_offset(0, 2),
            Color::Black => from.try_offset(0, -2),
        };

        from.rank() as u8 == start_rank
            && two_step == Some(to)
            && one_step.is_some_and(|square| board.get_piece_at(square) == Piece::None)
    }

    fn pseudolegal_castle(&self, board: &Board, m: Move) -> bool {
        let color = board.us();
        let king_from = match color {
            Color::White => Square::E1,
            Color::Black => Square::E8,
        };
        let king = match color {
            Color::White => Piece::WhiteKing,
            Color::Black => Piece::BlackKing,
        };

        if m.from() != king_from
            || board.get_piece_at(king_from) != king
            || self.is_attacked(board, king_from, !color)
        {
            return false;
        }

        let kind = match (color, m.kind(), m.to()) {
            (Color::White, MoveType::CastleKingside, Square::G1) => CastlingKind::WhiteKingside,
            (Color::White, MoveType::CastleQueenside, Square::C1) => CastlingKind::WhiteQueenside,
            (Color::Black, MoveType::CastleKingside, Square::G8) => CastlingKind::BlackKingside,
            (Color::Black, MoveType::CastleQueenside, Square::C8) => CastlingKind::BlackQueenside,
            _ => return false,
        };

        self.can_castle(board, color, kind)
    }

    fn gen_evasions(&self, board: &Board) -> MoveList {
        let mut ml = MoveList::default();
        let color = board.us();
        let king_square = Bitboard::square_from_bb(board.get_piece(PieceType::King, color));
        let checkers = self.attackers_to(board, king_square, board.them());

        debug_assert!(!checkers.is_empty());

        self.gen_king_moves::<Evasions>(board, &mut ml, color, Bitboard(u64::MAX));

        if checkers.count_ones() > 1 {
            return ml;
        }

        let checker_square = Bitboard::square_from_bb(checkers);
        let checker = board.get_piece_at(checker_square);
        let mut targets = checkers;
        if matches!(
            checker.ptype(),
            PieceType::Bishop | PieceType::Rook | PieceType::Queen
        ) {
            targets |= self.between[king_square][checker_square];
        }

        self.gen_knight_moves::<Evasions>(board, &mut ml, color, targets);
        self.gen_bishop_moves::<Evasions>(board, &mut ml, color, targets);
        self.gen_rook_moves::<Evasions>(board, &mut ml, color, targets);
        self.gen_queen_moves::<Evasions>(board, &mut ml, color, targets);
        self.gen_pawn_moves::<Evasions>(board, &mut ml, color, targets);

        ml
    }

    // ---------------------- MOVE GENERATION ---------------------

    fn gen_king_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_piece = board.get_piece(PieceType::King, color);

        for square in bb_piece {
            let attacks = self.king_moves[square];
            if G::QUIETS {
                self.add_moves(
                    ml,
                    square,
                    attacks & !board.get_occupancy() & targets,
                    MoveType::Quiet,
                );
            }
            if G::CAPTURES {
                self.add_moves(
                    ml,
                    square,
                    attacks & board.their_pieces() & targets,
                    MoveType::Capture,
                );
            }
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

    fn gen_knight_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_piece = board.get_piece(PieceType::Knight, color);
        for square in bb_piece {
            let attacks = self.knight_moves[square];
            if G::QUIETS {
                self.add_moves(
                    ml,
                    square,
                    attacks & !board.get_occupancy() & targets,
                    MoveType::Quiet,
                );
            }
            if G::CAPTURES {
                self.add_moves(
                    ml,
                    square,
                    attacks & board.their_pieces() & targets,
                    MoveType::Capture,
                );
            }
        }
    }

    fn gen_rook_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_piece = board.get_piece(PieceType::Rook, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let attacks = self.get_rook_attacks(square, bb_blockers);
            if G::QUIETS {
                self.add_moves(
                    ml,
                    square,
                    attacks & !bb_blockers & targets,
                    MoveType::Quiet,
                );
            }
            if G::CAPTURES {
                self.add_moves(
                    ml,
                    square,
                    attacks & board.their_pieces() & targets,
                    MoveType::Capture,
                );
            }
        }
    }

    fn gen_bishop_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_piece = board.get_piece(PieceType::Bishop, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let attacks = self.get_bishop_attacks(square, bb_blockers);
            if G::QUIETS {
                self.add_moves(
                    ml,
                    square,
                    attacks & !bb_blockers & targets,
                    MoveType::Quiet,
                );
            }
            if G::CAPTURES {
                self.add_moves(
                    ml,
                    square,
                    attacks & board.their_pieces() & targets,
                    MoveType::Capture,
                );
            }
        }
    }

    fn gen_queen_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_piece = board.get_piece(PieceType::Queen, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let attacks = self.get_queen_attacks(square, bb_blockers);
            if G::QUIETS {
                self.add_moves(
                    ml,
                    square,
                    attacks & !bb_blockers & targets,
                    MoveType::Quiet,
                );
            }
            if G::CAPTURES {
                self.add_moves(
                    ml,
                    square,
                    attacks & board.their_pieces() & targets,
                    MoveType::Capture,
                );
            }
        }
    }

    fn gen_pawn_moves<G: GenType>(
        &self,
        board: &Board,
        ml: &mut MoveList,
        color: Color,
        targets: Bitboard,
    ) {
        let bb_empty = !board.get_occupancy();
        let bb_promo_rank = Bitboard::promotion_rank(color);
        let bb_fourth_rank = Bitboard::fourth_rank(color);
        let bb_opponent = board.their_pieces();
        let ep_square = board.get_ep_square();
        let bb_ep_square = match ep_square {
            Square::None => Bitboard(0),
            _ => Bitboard::from_square(ep_square),
        };
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

            let push = one_step & !bb_promo_rank & targets;
            let double_push = two_steps & targets;
            let promos = one_step & bb_promo_rank & targets;

            let moves = self.pawn_attacks[color][square];
            let captures = moves & bb_opponent & !bb_promo_rank & targets;
            let ep_captures = moves & bb_ep_square;
            let promo_captures = moves & bb_opponent & bb_promo_rank & targets;

            if G::QUIETS {
                self.add_moves(ml, square, push, MoveType::Quiet);
                self.add_moves(ml, square, double_push, MoveType::DoublePawnPush);
            }
            if G::CAPTURES {
                self.add_moves(ml, square, captures, MoveType::Capture);
                let ep_targets = if G::EVASIONS {
                    ep_captures
                } else {
                    ep_captures & targets
                };
                self.add_moves(ml, square, ep_targets, MoveType::EnPassant);
            }
            if G::PROMOTIONS {
                self.add_promotion(ml, square, promos, false);
                self.add_promotion(ml, square, promo_captures, true);
            }
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

    fn add_moves(&self, ml: &mut MoveList, from: Square, to: Bitboard, kind: MoveType) {
        for square in to {
            let m = Move::new(from, square, kind);
            ml.push(m);
        }
    }

    pub fn get_pawn_attack_mask(&self, square: Square, color: Color) -> Bitboard {
        self.pawn_attacks[color][square]
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

    fn init_between(&mut self) {
        for from_idx in 0..Square::COUNT {
            let from = Square::from_idx(from_idx);
            for to_idx in 0..Square::COUNT {
                let to = Square::from_idx(to_idx);
                let file_delta = to.file() as i8 - from.file() as i8;
                let rank_delta = to.rank() as i8 - from.rank() as i8;

                if file_delta != 0 && rank_delta != 0 && file_delta.abs() != rank_delta.abs() {
                    continue;
                }

                let file_step = file_delta.signum();
                let rank_step = rank_delta.signum();
                if file_step == 0 && rank_step == 0 {
                    continue;
                }

                let mut square = from;
                while let Some(next) = square.try_offset(file_step, rank_step) {
                    if next == to {
                        break;
                    }
                    self.between[from][to].set(next);
                    square = next;
                }
            }
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

    pub fn attackers_to(&self, board: &Board, square: Square, color: Color) -> Bitboard {
        let attackers = board.bitboards[color];
        let occupancy = board.get_occupancy();

        let rooks_queen = attackers[PieceType::Rook] | attackers[PieceType::Queen];
        let bishops_queen = attackers[PieceType::Bishop] | attackers[PieceType::Queen];

        (self.get_rook_attacks(square, occupancy) & rooks_queen)
            | (self.get_bishop_attacks(square, occupancy) & bishops_queen)
            | (self.knight_moves[square] & attackers[PieceType::Knight])
            | (self.pawn_attacks[!color][square] & attackers[PieceType::Pawn])
            | (self.king_moves[square] & attackers[PieceType::King])
    }

    pub fn is_attacked(&self, board: &Board, square: Square, color: Color) -> bool {
        !self.attackers_to(board, square, color).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn move_values(moves: &MoveList) -> Vec<u16> {
        let mut values = (0..moves.len())
            .map(|idx| moves.get(idx).0)
            .collect::<Vec<_>>();
        values.sort_unstable();
        values
    }

    fn is_promotion(m: Move) -> bool {
        matches!(
            m.kind(),
            MoveType::NPromotion
                | MoveType::BPromotion
                | MoveType::RPromotion
                | MoveType::QPromotion
                | MoveType::NPromoCapture
                | MoveType::BPromoCapture
                | MoveType::RPromoCapture
                | MoveType::QPromoCapture
        )
    }

    fn legal_move_values<G: GenType>(board: &Board, movegen: &MoveGenerator) -> Vec<u16> {
        let moves = movegen.gen_moves::<G>(board);
        let mut board = board.clone();
        let mut values = Vec::new();

        for idx in 0..moves.len() {
            let m = moves.get(idx);
            if board.make(m, movegen) {
                values.push(m.0);
                board.unmake();
            }
        }

        values.sort_unstable();
        values
    }

    #[test]
    fn between_contains_only_aligned_intermediate_squares() {
        let movegen = MoveGenerator::new();

        assert_eq!(
            movegen.between[Square::A1][Square::A4],
            Bitboard::from_square(Square::A2) | Bitboard::from_square(Square::A3)
        );
        assert_eq!(
            movegen.between[Square::A1][Square::D4],
            Bitboard::from_square(Square::B2) | Bitboard::from_square(Square::C3)
        );
        assert_eq!(
            movegen.between[Square::D4][Square::A1],
            Bitboard::from_square(Square::C3) | Bitboard::from_square(Square::B2)
        );
        assert!(movegen.between[Square::A1][Square::B2].is_empty());
        assert!(movegen.between[Square::A1][Square::B3].is_empty());
    }

    #[test]
    fn attackers_to_returns_all_attackers() {
        let board = Board::from_fen("4r2k/1b6/5n2/5p2/4K3/8/8/8 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let attackers = movegen.attackers_to(&board, Square::E4, Color::Black);

        assert_eq!(attackers.count_ones(), 4);
        assert!(attackers.contains(Square::E8));
        assert!(attackers.contains(Square::B7));
        assert!(attackers.contains(Square::F6));
        assert!(attackers.contains(Square::F5));
    }

    #[test]
    fn double_check_evasions_generate_only_king_moves() {
        let board = Board::from_fen("4r1k1/8/8/8/1b6/8/R7/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let moves = movegen.gen_moves::<Evasions>(&board);

        assert!((0..moves.len()).all(|idx| moves.get(idx).from() == Square::E1));
    }

    #[test]
    fn evasion_legal_moves_match_all_legal_moves() {
        let movegen = MoveGenerator::new();
        let positions = [
            "4r1k1/8/8/8/8/8/3B4/4K3 w - - 0 1",
            "4k3/8/8/8/8/5n2/6B1/4K3 w - - 0 1",
            "4r1k1/8/8/8/1b6/8/R7/4K3 w - - 0 1",
            "4k3/8/8/3pP3/8/8/8/4K2r w - d6 0 1",
        ];

        for fen in positions {
            let board = Board::from_fen(fen).unwrap();
            assert_eq!(
                legal_move_values::<Evasions>(&board, &movegen),
                legal_move_values::<All>(&board, &movegen),
                "{fen}"
            );
        }
    }

    #[test]
    fn all_moves_are_partitioned_into_noisy_and_quiet() {
        let board = Board::from_fen("1r2k3/P6r/8/3pP3/8/8/8/R3K2R w KQ d6 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let all = movegen.gen_moves::<All>(&board);
        let noisy = movegen.gen_moves::<Noisy>(&board);
        let quiet = movegen.gen_moves::<Quiet>(&board);
        let mut staged = move_values(&noisy);
        staged.extend(move_values(&quiet));
        staged.sort_unstable();

        assert_eq!(move_values(&all), staged);
    }

    #[test]
    fn noisy_moves_include_only_captures_and_promotions() {
        let board = Board::from_fen("1r2k3/P6r/8/3pP3/8/8/8/R3K2R w KQ d6 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let moves = movegen.gen_moves::<Noisy>(&board);

        assert!((0..moves.len()).all(|idx| {
            let m = moves.get(idx);
            m.is_capture() || is_promotion(m)
        }));
        assert!((0..moves.len()).any(|idx| moves.get(idx).kind() == MoveType::EnPassant));
        assert!((0..moves.len()).any(|idx| {
            let m = moves.get(idx);
            is_promotion(m) && !m.is_capture()
        }));
        assert!((0..moves.len()).any(|idx| {
            let m = moves.get(idx);
            is_promotion(m) && m.is_capture()
        }));
    }

    #[test]
    fn quiet_moves_exclude_captures_and_promotions() {
        let board = Board::from_fen("1r2k3/P6r/8/3pP3/8/8/8/R3K2R w KQ d6 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let moves = movegen.gen_moves::<Quiet>(&board);

        assert!((0..moves.len()).all(|idx| {
            let m = moves.get(idx);
            !m.is_capture() && !is_promotion(m)
        }));
        assert!((0..moves.len()).any(|idx| {
            matches!(
                moves.get(idx).kind(),
                MoveType::CastleKingside | MoveType::CastleQueenside
            )
        }));
    }

    #[test]
    fn is_pseudolegal_killer_accepts_generated_quiet_move() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let m = Move::new(Square::E2, Square::E4, MoveType::DoublePawnPush);

        assert!(movegen.is_pseudolegal_killer(&board, m));
    }

    #[test]
    fn is_pseudolegal_killer_rejects_move_from_empty_square() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let m = Move::new(Square::D2, Square::D4, MoveType::DoublePawnPush);

        assert!(!movegen.is_pseudolegal_killer(&board, m));
    }

    #[test]
    fn is_pseudolegal_killer_rejects_blocked_quiet_move() {
        let board = Board::from_fen("4k3/8/8/8/8/4P3/4P3/4K3 w - - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let m = Move::new(Square::E2, Square::E4, MoveType::DoublePawnPush);

        assert!(!movegen.is_pseudolegal_killer(&board, m));
    }

    #[test]
    fn is_pseudolegal_killer_accepts_castling_when_generated() {
        let board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let movegen = MoveGenerator::new();
        let m = Move::new(Square::E1, Square::G1, MoveType::CastleKingside);

        assert!(movegen.is_pseudolegal_killer(&board, m));
    }

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

        assert_eq!(movegen.gen_moves::<All>(&board).len(), 20);
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
        let moves = movegen.gen_moves::<All>(&board);
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
