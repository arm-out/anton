use std::mem::size_of;

use crate::{evaluation::Score, movegen::moves::Move};

pub const DEFAULT_TT_SIZE_MB: usize = 256;

const BYTES_PER_MIB: usize = 1024 * 1024;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) enum Bound {
    Exact,
    Lower,
    Upper,
}

impl Default for Bound {
    fn default() -> Self {
        Self::Exact
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(super) struct TTEntry {
    key: u64,
    best_move: Move,
    score: Score,
    depth: u8,
    bound: Bound,
    valid: bool,
}

impl TTEntry {
    pub(super) fn new(key: u64, best_move: Move, score: Score, depth: u8, bound: Bound) -> Self {
        Self {
            key,
            best_move,
            score,
            depth,
            bound,
            valid: true,
        }
    }

    pub(super) fn best_move(&self) -> Move {
        self.best_move
    }

    pub(super) fn score(&self) -> Score {
        self.score
    }

    pub(super) fn depth(&self) -> u8 {
        self.depth
    }

    pub(super) fn bound(&self) -> Bound {
        self.bound
    }
}

pub(super) struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    pub(super) fn new(size_mb: usize) -> Self {
        let bytes = size_mb.saturating_mul(BYTES_PER_MIB);
        let capacity = (bytes / size_of::<TTEntry>()).max(1);
        let next_power_of_two = capacity.next_power_of_two();
        let len = if next_power_of_two == capacity {
            capacity
        } else {
            next_power_of_two >> 1
        };

        Self {
            entries: vec![TTEntry::default(); len],
        }
    }

    pub(super) fn probe(&self, key: u64) -> Option<TTEntry> {
        let entry = self.entries[self.index(key)];

        if entry.valid && entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    pub(super) fn store(
        &mut self,
        key: u64,
        best_move: Move,
        score: Score,
        depth: u8,
        bound: Bound,
    ) {
        let index = self.index(key);
        let entry = self.entries[index];

        if !entry.valid || entry.key == key || depth >= entry.depth {
            self.entries[index] = TTEntry::new(key, best_move, score, depth, bound);
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    fn index(&self, key: u64) -> usize {
        key as usize & (self.entries.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::square::Square,
        movegen::moves::{Move, MoveType},
    };

    #[test]
    fn table_size_calculation_creates_power_of_two_entries() {
        assert_eq!(TranspositionTable::new(0).len(), 1);
        let len = TranspositionTable::new(1).len();

        assert!(len > 1);
        assert!(len.is_power_of_two());
    }

    #[test]
    fn same_key_probes_successfully() {
        let mut table = TranspositionTable::new(1);
        let best_move = Move::new(Square::E2, Square::E4, MoveType::Quiet);

        table.store(42, best_move, 123, 4, Bound::Exact);

        let entry = table.probe(42).unwrap();
        assert_eq!(entry.best_move(), best_move);
        assert_eq!(entry.score(), 123);
        assert_eq!(entry.depth(), 4);
        assert_eq!(entry.bound(), Bound::Exact);
    }

    #[test]
    fn different_key_at_same_index_does_not_match() {
        let mut table = TranspositionTable::new(0);
        let best_move = Move::new(Square::E2, Square::E4, MoveType::Quiet);

        table.store(1, best_move, 123, 4, Bound::Exact);

        assert!(table.probe(2).is_none());
    }

    #[test]
    fn deeper_entries_are_not_replaced_by_shallower_unrelated_entries() {
        let mut table = TranspositionTable::new(0);
        let deep_move = Move::new(Square::E2, Square::E4, MoveType::Quiet);
        let shallow_move = Move::new(Square::D2, Square::D4, MoveType::Quiet);

        table.store(1, deep_move, 100, 5, Bound::Exact);
        table.store(2, shallow_move, 200, 3, Bound::Exact);

        let entry = table.probe(1).unwrap();
        assert_eq!(entry.best_move(), deep_move);
        assert_eq!(entry.score(), 100);
    }
}
