use crate::{
    board::{
        Board,
        piece::{Color, Piece},
        square::Square,
    },
    movegen::moves::Move,
};

pub(super) const MAX_HISTORY: i16 = 16_384;

type ButterflyHistory = [[[i16; Square::COUNT]; Square::COUNT]; Color::COUNT];
type ContinuationHistory = Vec<i16>;
const CONTINUATION_HISTORY_SIZE: usize =
    Piece::COUNT * Square::COUNT * Piece::COUNT * Square::COUNT;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) struct StackEntry {
    piece: Piece,
    to: Square,
    valid: bool,
}

impl Default for StackEntry {
    fn default() -> Self {
        Self {
            piece: Piece::None,
            to: Square::None,
            valid: false,
        }
    }
}

pub(super) type SearchStack = [StackEntry; super::MAX_SEARCH_DEPTH as usize];

pub(super) struct History {
    butterfly: ButterflyHistory,
    continuation_1ply: ContinuationHistory,
    continuation_2ply: ContinuationHistory,
}

#[derive(Copy, Clone)]
pub(super) struct HistoryView<'a> {
    history: &'a History,
    stack: &'a SearchStack,
    ply: u8,
}

impl History {
    pub(super) fn new() -> Self {
        Self {
            butterfly: [[[0; Square::COUNT]; Square::COUNT]; Color::COUNT],
            continuation_1ply: vec![0; CONTINUATION_HISTORY_SIZE],
            continuation_2ply: vec![0; CONTINUATION_HISTORY_SIZE],
        }
    }

    pub(super) fn clear(&mut self) {
        self.butterfly = [[[0; Square::COUNT]; Square::COUNT]; Color::COUNT];
        self.continuation_1ply.fill(0);
        self.continuation_2ply.fill(0);
    }

    pub(super) fn view<'a>(&'a self, stack: &'a SearchStack, ply: u8) -> HistoryView<'a> {
        HistoryView {
            history: self,
            stack,
            ply,
        }
    }

    pub(super) fn update(
        &mut self,
        stack: &SearchStack,
        ply: u8,
        board: &Board,
        color: Color,
        m: Move,
        depth: u8,
        searched_quiets: &[Move],
    ) {
        let bonus = history_bonus(depth);

        self.update_move(stack, ply, board, color, m, bonus);

        for quiet in searched_quiets {
            self.update_move(stack, ply, board, color, *quiet, -bonus);
        }
    }

    fn update_move(
        &mut self,
        stack: &SearchStack,
        ply: u8,
        board: &Board,
        color: Color,
        m: Move,
        bonus: i16,
    ) {
        let from = m.from();
        let to = m.to();

        // update butterfly history
        update_entry(&mut self.butterfly[color][from][to], bonus);

        let piece = board.get_piece_at(from);
        let current = StackEntry {
            piece,
            to: m.to(),
            valid: piece != Piece::None,
        };

        if let Some(previous) = previous_entry(stack, ply, 1) {
            update_continuation_entry(&mut self.continuation_1ply, previous, current, bonus);
        }

        if let Some(previous) = previous_entry(stack, ply, 2) {
            update_continuation_entry(&mut self.continuation_2ply, previous, current, bonus);
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> HistoryView<'a> {
    pub(super) fn quiet_score(self, board: &Board, m: Move) -> i32 {
        let piece = board.get_piece_at(m.from());
        let current = StackEntry {
            piece,
            to: m.to(),
            valid: piece != Piece::None,
        };
        let history = i32::from(self.history.butterfly[board.us()][m.from()][m.to()]);
        let continuation1 = continuation_score_at(
            &self.history.continuation_1ply,
            self.stack,
            self.ply,
            1,
            current,
        );
        let continuation2 = continuation_score_at(
            &self.history.continuation_2ply,
            self.stack,
            self.ply,
            2,
            current,
        );

        history + continuation1 / 2 + continuation2 / 4
    }
}

pub(super) fn clear_stack(stack: &mut SearchStack) {
    *stack = new_stack();
}

pub(super) fn new_stack() -> SearchStack {
    [StackEntry::default(); super::MAX_SEARCH_DEPTH as usize]
}

pub(super) fn set_stack_move(stack: &mut SearchStack, ply: u8, piece: Piece, m: Move) {
    let Some(entry) = stack.get_mut(ply as usize) else {
        return;
    };

    *entry = StackEntry {
        piece,
        to: m.to(),
        valid: piece != Piece::None,
    };
}

pub(super) fn clear_stack_entry(stack: &mut SearchStack, ply: u8) {
    let Some(entry) = stack.get_mut(ply as usize) else {
        return;
    };

    *entry = StackEntry::default();
}

fn previous_entry(stack: &SearchStack, ply: u8, back: u8) -> Option<StackEntry> {
    let idx = ply.checked_sub(back)? as usize;
    let entry = *stack.get(idx)?;

    entry.valid.then_some(entry)
}

fn update_continuation_entry(
    history: &mut ContinuationHistory,
    previous: StackEntry,
    current: StackEntry,
    bonus: i16,
) {
    if !current.valid {
        return;
    }

    update_entry(&mut history[continuation_idx(previous, current)], bonus);
}

fn continuation_score(
    history: &ContinuationHistory,
    previous: StackEntry,
    current: StackEntry,
) -> i16 {
    if !current.valid {
        return 0;
    }

    history[continuation_idx(previous, current)]
}

fn continuation_score_at(
    history: &ContinuationHistory,
    stack: &SearchStack,
    ply: u8,
    back: u8,
    current: StackEntry,
) -> i32 {
    previous_entry(stack, ply, back)
        .map(|previous| i32::from(continuation_score(history, previous, current)))
        .unwrap_or(0)
}

fn continuation_idx(previous: StackEntry, current: StackEntry) -> usize {
    (((previous.piece as usize * Square::COUNT + previous.to as usize) * Piece::COUNT
        + current.piece as usize)
        * Square::COUNT)
        + current.to as usize
}

fn history_bonus(depth: u8) -> i16 {
    (16 * i32::from(depth) * i32::from(depth)).min(2000) as i16
}

fn update_entry(entry: &mut i16, bonus: i16) {
    let gravity = i32::from(*entry) * i32::from(bonus.abs()) / i32::from(MAX_HISTORY);
    let updated = i32::from(*entry) + i32::from(bonus) - gravity;

    *entry = updated.clamp(-i32::from(MAX_HISTORY), i32::from(MAX_HISTORY)) as i16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::moves::{Move, MoveType};

    fn stack_with_context() -> SearchStack {
        let mut stack = new_stack();

        set_stack_move(
            &mut stack,
            1,
            Piece::WhiteKnight,
            Move::new(Square::G1, Square::F3, MoveType::Quiet),
        );
        set_stack_move(
            &mut stack,
            2,
            Piece::BlackKnight,
            Move::new(Square::G8, Square::F6, MoveType::Quiet),
        );

        stack
    }

    #[test]
    fn history_bonus_is_capped() {
        assert_eq!(history_bonus(1), 16);
        assert_eq!(history_bonus(12), 2000);
    }

    #[test]
    fn butterfly_history_uses_gravity_bounds() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);
        let stack = new_stack();

        for _ in 0..64 {
            history.update(&stack, 0, &board, Color::White, m, 12, &[]);
        }

        assert!(history.butterfly[Color::White][Square::E2][Square::E4] <= MAX_HISTORY);
        assert!(history.butterfly[Color::White][Square::E2][Square::E4] > 0);

        for _ in 0..128 {
            history.update(&stack, 0, &board, Color::White, m, 12, &[m]);
        }

        assert!(history.butterfly[Color::White][Square::E2][Square::E4] >= -MAX_HISTORY);
        assert!(history.butterfly[Color::White][Square::E2][Square::E4] < 0);

        assert_eq!(board.history.len(), 0);
    }

    #[test]
    fn continuation_history_uses_gravity_bounds() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);
        let stack = stack_with_context();

        for _ in 0..64 {
            history.update(&stack, 3, &board, Color::White, m, 12, &[]);
        }

        assert!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )] <= MAX_HISTORY
        );
        assert!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )] > 0
        );
    }

    #[test]
    fn clear_resets_all_history_tables() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);
        let stack = stack_with_context();

        history.update(&stack, 3, &board, Color::White, m, 2, &[]);
        assert_ne!(history.butterfly[Color::White][Square::E2][Square::E4], 0);
        assert_ne!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );
        assert_ne!(
            history.continuation_2ply[continuation_idx(
                StackEntry {
                    piece: Piece::WhiteKnight,
                    to: Square::F3,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );

        history.clear();

        assert_eq!(history.butterfly[Color::White][Square::E2][Square::E4], 0);
        assert_eq!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );
        assert_eq!(
            history.continuation_2ply[continuation_idx(
                StackEntry {
                    piece: Piece::WhiteKnight,
                    to: Square::F3,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );
    }

    #[test]
    fn update_rewards_cutoff_and_maluses_searched_quiets() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/3PP3/4K1N1 w - - 0 1").unwrap();
        let stack = stack_with_context();
        let cutoff = Move::new(Square::E2, Square::E4, MoveType::Quiet);
        let searched = [
            Move::new(Square::D2, Square::D4, MoveType::Quiet),
            Move::new(Square::G1, Square::F3, MoveType::Quiet),
        ];

        history.update(&stack, 3, &board, Color::White, cutoff, 2, &searched);

        assert_eq!(history.butterfly[Color::White][Square::E2][Square::E4], 64);
        assert_eq!(history.butterfly[Color::White][Square::D2][Square::D4], -64);
        assert_eq!(history.butterfly[Color::White][Square::G1][Square::F3], -64);
        assert_eq!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            64
        );
        assert_eq!(
            history.continuation_2ply[continuation_idx(
                StackEntry {
                    piece: Piece::WhiteKnight,
                    to: Square::F3,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            64
        );
    }

    #[test]
    fn invalid_stack_entries_only_update_butterfly() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let stack = new_stack();
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);

        history.update(&stack, 3, &board, Color::White, m, 2, &[]);

        assert_eq!(history.butterfly[Color::White][Square::E2][Square::E4], 64);
        assert_eq!(
            history.continuation_1ply[continuation_idx(
                StackEntry {
                    piece: Piece::BlackKnight,
                    to: Square::F6,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );
        assert_eq!(
            history.continuation_2ply[continuation_idx(
                StackEntry {
                    piece: Piece::WhiteKnight,
                    to: Square::F3,
                    valid: true,
                },
                StackEntry {
                    piece: Piece::WhitePawn,
                    to: Square::E4,
                    valid: true,
                },
            )],
            0
        );
    }

    #[test]
    fn quiet_score_combines_butterfly_and_continuations() {
        let mut history = History::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let stack = stack_with_context();
        let m = Move::new(Square::E2, Square::E4, MoveType::Quiet);

        history.butterfly[Color::White][Square::E2][Square::E4] = 10;
        history.continuation_1ply[continuation_idx(
            StackEntry {
                piece: Piece::BlackKnight,
                to: Square::F6,
                valid: true,
            },
            StackEntry {
                piece: Piece::WhitePawn,
                to: Square::E4,
                valid: true,
            },
        )] = 20;
        history.continuation_2ply[continuation_idx(
            StackEntry {
                piece: Piece::WhiteKnight,
                to: Square::F3,
                valid: true,
            },
            StackEntry {
                piece: Piece::WhitePawn,
                to: Square::E4,
                valid: true,
            },
        )] = 30;

        assert_eq!(history.view(&stack, 3).quiet_score(&board, m), 60);
    }
}
