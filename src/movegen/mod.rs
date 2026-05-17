use crate::{
    board::{
        Board,
        bitboard::Bitboard,
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
pub mod magic;
mod movelist;
mod moves;

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
            self.add_moves(
                board,
                ml,
                square,
                bb_moves,
                PieceType::King,
                MoveType::Quiet,
            );
        }
    }

    fn gen_knight_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Knight, color);
        for square in bb_piece {
            let bb_moves = self.knight_moves[square] & mask;
            self.add_moves(
                board,
                ml,
                square,
                bb_moves,
                PieceType::Knight,
                MoveType::Quiet,
            );
        }
    }

    fn gen_rook_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Rook, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bb_moves = self.get_rook_attacks(square, bb_blockers) & mask;
            self.add_moves(
                board,
                ml,
                square,
                bb_moves,
                PieceType::Rook,
                MoveType::Quiet,
            );
        }
    }

    fn gen_bishop_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Bishop, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bb_moves = self.get_bishop_attacks(square, bb_blockers) & mask;
            self.add_moves(
                board,
                ml,
                square,
                bb_moves,
                PieceType::Bishop,
                MoveType::Quiet,
            );
        }
    }

    fn gen_queen_moves(&self, board: &Board, ml: &mut MoveList, mask: Bitboard, color: Color) {
        let bb_piece = board.get_piece(PieceType::Queen, color);
        let bb_blockers = board.get_occupancy();

        for square in bb_piece {
            let bb_moves = self.get_queen_attacks(square, bb_blockers) & mask;
            self.add_moves(
                board,
                ml,
                square,
                bb_moves,
                PieceType::Queen,
                MoveType::Quiet,
            );
        }
    }

    fn gen_pawn_pushes(&self, board: &Board, ml: &mut MoveList, color: Color) {
        let bb_empty = !board.get_occupancy();
        let bb_promo_rank = Bitboard::promotion_rank(color);
        let bb_fourth_rank = Bitboard::fourth_rank(color);
        let dir = if color == Color::White { 8 } else { -8 };
        let bb_pieces = board.get_piece(PieceType::Pawn, color);

        for square in bb_pieces {
            // Single push
            let push_square = square + dir;
            let one_step = Bitboard::from_square(push_square) & bb_empty;
            let two_steps = if dir > 0 {
                (one_step << dir) & bb_empty & bb_fourth_rank
            } else {
                (one_step >> dir) & bb_empty & bb_fourth_rank
            };

            let push = one_step & !bb_promo_rank;
            let double_push = two_steps;
            let promos = one_step & bb_promo_rank;

            self.add_moves(board, ml, square, push, PieceType::Pawn, MoveType::Quiet);
            self.add_moves(
                board,
                ml,
                square,
                double_push,
                PieceType::Pawn,
                MoveType::DoublePawnPush,
            );
            self.add_promotion(ml, square, promos, false);
        }
    }

    fn gen_pawn_captures(&self, board: &Board, ml: &mut MoveList, color: Color) {
        let bb_opponent = board.their_pieces();
        let bb_promo_rank = Bitboard::promotion_rank(color);
        let bb_ep_square = Bitboard::from_square(board.get_ep_square());
        let bb_pieces = board.get_piece(PieceType::Pawn, color);

        for square in bb_pieces {
            let moves = self.pawn_attacks[color][square];
            let captures = moves & bb_opponent & !bb_promo_rank;
            let ep_captures = moves & bb_ep_square;
            let promo_captures = moves & bb_opponent & bb_promo_rank;

            self.add_moves(
                board,
                ml,
                square,
                captures,
                PieceType::Pawn,
                MoveType::Quiet,
            );
            self.add_moves(
                board,
                ml,
                square,
                ep_captures,
                PieceType::Pawn,
                MoveType::EnPassant,
            );
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
        ptype: PieceType,
        kind: MoveType,
    ) {
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
        let idx = MoveGenerator::magic_index(blockers, &self.rook_magics[square]);
        self.rook_moves[idx]
    }

    fn get_bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let idx = MoveGenerator::magic_index(blockers, &self.bishop_magics[square]);
        self.bishop_moves[idx]
    }

    fn get_queen_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let r_idx = MoveGenerator::magic_index(blockers, &self.rook_magics[square]);
        let b_idx = MoveGenerator::magic_index(blockers, &self.bishop_magics[square]);
        self.rook_moves[r_idx] ^ self.bishop_moves[b_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
