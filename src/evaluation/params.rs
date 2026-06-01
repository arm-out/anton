use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use crate::board::{
    piece::{Piece, PieceType},
    square::Square,
};

use super::{EvalValue, MAX_GAME_PHASE, Score, eval_to_score, weights};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvalScore {
    pub mg: EvalValue,
    pub eg: EvalValue,
}

impl EvalScore {
    pub const fn new(mg: EvalValue, eg: EvalValue) -> Self {
        Self { mg, eg }
    }

    pub const fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn tapered(self, game_phase: u8) -> Score {
        let phase = game_phase.min(MAX_GAME_PHASE) as EvalValue;
        let eg_phase = MAX_GAME_PHASE as EvalValue - phase;
        eval_to_score((self.mg * phase + self.eg * eg_phase) / MAX_GAME_PHASE as EvalValue)
    }
}

impl Add for EvalScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }
}

impl AddAssign for EvalScore {
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub for EvalScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl SubAssign for EvalScore {
    fn sub_assign(&mut self, rhs: Self) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl Mul<EvalValue> for EvalScore {
    type Output = Self;

    fn mul(self, rhs: EvalValue) -> Self::Output {
        Self::new(self.mg * rhs, self.eg * rhs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvalParams {
    pub material: [EvalScore; PieceType::COUNT],
    pub psqt: [[EvalScore; Square::COUNT]; PieceType::COUNT],
    pub piece_square: [[EvalScore; Square::COUNT]; PieceType::COUNT],
}

impl EvalParams {
    pub const fn current() -> Self {
        Self::from_weights(weights::WEIGHTS)
    }

    pub const fn from_weights(weights: [EvalValue; weights::WEIGHT_COUNT]) -> Self {
        let mut params = Self {
            material: [EvalScore::zero(); PieceType::COUNT],
            psqt: [[EvalScore::zero(); Square::COUNT]; PieceType::COUNT],
            piece_square: [[EvalScore::zero(); Square::COUNT]; PieceType::COUNT],
        };
        let mut ptype = 0;

        while ptype < PieceType::COUNT {
            params.material[ptype] = EvalScore::new(
                weights[material_mg_weight_idx(ptype)],
                weights[material_eg_weight_idx(ptype)],
            );

            let mut square = 0;
            while square < Square::COUNT {
                params.psqt[ptype][square] = EvalScore::new(
                    weights[psqt_mg_weight_idx(ptype, square)],
                    weights[psqt_eg_weight_idx(ptype, square)],
                );
                params.piece_square[ptype][square] = EvalScore::new(
                    params.material[ptype].mg + params.psqt[ptype][square].mg,
                    params.material[ptype].eg + params.psqt[ptype][square].eg,
                );
                square += 1;
            }

            ptype += 1;
        }

        params
    }

    pub const fn weights(&self) -> [EvalValue; weights::WEIGHT_COUNT] {
        let mut weights = [0; weights::WEIGHT_COUNT];
        let mut ptype = 0;

        while ptype < PieceType::COUNT {
            weights[material_mg_weight_idx(ptype)] = self.material[ptype].mg;
            weights[material_eg_weight_idx(ptype)] = self.material[ptype].eg;

            let mut square = 0;
            while square < Square::COUNT {
                weights[psqt_mg_weight_idx(ptype, square)] = self.psqt[ptype][square].mg;
                weights[psqt_eg_weight_idx(ptype, square)] = self.psqt[ptype][square].eg;
                square += 1;
            }

            ptype += 1;
        }

        weights
    }

    pub fn piece_square_value(&self, square: Square, piece: Piece) -> EvalScore {
        let ptype = piece.ptype();
        if ptype == PieceType::None {
            return EvalScore::zero();
        }

        self.piece_square[ptype][square.psqt_idx(piece.color())]
    }
}

pub const EVAL_PARAMS: EvalParams = EvalParams::current();

#[derive(Clone, Copy, Debug, Default)]
pub struct EvalContext;

pub const fn material_mg_weight_idx(ptype: usize) -> usize {
    ptype
}

pub const fn material_eg_weight_idx(ptype: usize) -> usize {
    PieceType::COUNT + ptype
}

pub const fn psqt_mg_weight_idx(ptype: usize, square: usize) -> usize {
    2 * PieceType::COUNT + ptype * Square::COUNT + square
}

pub const fn psqt_eg_weight_idx(ptype: usize, square: usize) -> usize {
    2 * PieceType::COUNT + PieceType::COUNT * Square::COUNT + ptype * Square::COUNT + square
}
