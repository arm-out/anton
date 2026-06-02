use crate::board::piece::{Color, PieceType};

use super::{
    EvalValue, MAX_GAME_PHASE,
    params::{
        doubled_pawn_eg_weight_idx, doubled_pawn_mg_weight_idx, isolated_pawn_eg_weight_idx,
        isolated_pawn_mg_weight_idx, material_eg_weight_idx, material_mg_weight_idx,
        psqt_eg_weight_idx, psqt_mg_weight_idx,
    },
    weights,
};

pub const FEATURE_COUNT: usize = weights::WEIGHT_COUNT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EvalTerm {
    Material(PieceType),
    Psqt(PieceType, usize),
    IsolatedPawn(usize),
    DoubledPawn(usize),
}

impl EvalTerm {
    pub fn mg_feature_idx(self) -> usize {
        match self {
            Self::Material(ptype) => material_mg_weight_idx(ptype as usize),
            Self::Psqt(ptype, psqt_idx) => psqt_mg_weight_idx(ptype as usize, psqt_idx),
            Self::IsolatedPawn(file) => isolated_pawn_mg_weight_idx(file),
            Self::DoubledPawn(file) => doubled_pawn_mg_weight_idx(file),
        }
    }

    pub fn eg_feature_idx(self) -> usize {
        match self {
            Self::Material(ptype) => material_eg_weight_idx(ptype as usize),
            Self::Psqt(ptype, psqt_idx) => psqt_eg_weight_idx(ptype as usize, psqt_idx),
            Self::IsolatedPawn(file) => isolated_pawn_eg_weight_idx(file),
            Self::DoubledPawn(file) => doubled_pawn_eg_weight_idx(file),
        }
    }
}

pub trait Trace {
    const USE_INCREMENTAL_EVAL: bool = false;

    fn term(&mut self, side: Color, term: EvalTerm, count: EvalValue);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTrace;

impl Trace for NoTrace {
    const USE_INCREMENTAL_EVAL: bool = true;

    #[inline(always)]
    fn term(&mut self, _side: Color, _term: EvalTerm, _count: EvalValue) {}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureVectorTrace {
    pub features: Vec<EvalValue>,
}

impl Default for FeatureVectorTrace {
    fn default() -> Self {
        Self {
            features: vec![0; FEATURE_COUNT],
        }
    }
}

impl FeatureVectorTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn material_feature(&self, ptype: PieceType) -> EvalValue {
        self.features[EvalTerm::Material(ptype).mg_feature_idx()]
    }

    pub fn psqt_feature(&self, ptype: PieceType, psqt_idx: usize) -> EvalValue {
        self.features[EvalTerm::Psqt(ptype, psqt_idx).mg_feature_idx()]
    }

    pub fn isolated_pawn_feature(&self, file: usize) -> EvalValue {
        self.features[EvalTerm::IsolatedPawn(file).mg_feature_idx()]
    }

    pub fn doubled_pawn_feature(&self, file: usize) -> EvalValue {
        self.features[EvalTerm::DoubledPawn(file).mg_feature_idx()]
    }

    pub fn tapered_features(&self, game_phase: u8) -> Vec<f64> {
        let phase = game_phase.min(MAX_GAME_PHASE) as f64;
        let max_phase = MAX_GAME_PHASE as f64;
        let mg_scale = phase / max_phase;
        let eg_scale = (max_phase - phase) / max_phase;

        self.features
            .iter()
            .enumerate()
            .map(|(idx, &count)| {
                let scale = if is_mg_feature_idx(idx) {
                    mg_scale
                } else {
                    eg_scale
                };

                count as f64 * scale
            })
            .collect()
    }
}

impl Trace for FeatureVectorTrace {
    fn term(&mut self, side: Color, term: EvalTerm, count: EvalValue) {
        let signed_count = trace_sign(side) * count;
        self.features[term.mg_feature_idx()] += signed_count;
        self.features[term.eg_feature_idx()] += signed_count;
    }
}

fn trace_sign(side: Color) -> EvalValue {
    match side {
        Color::White => 1,
        Color::Black => -1,
    }
}

fn is_mg_feature_idx(idx: usize) -> bool {
    idx < material_eg_weight_idx(0)
        || (psqt_mg_weight_idx(0, 0)..psqt_eg_weight_idx(0, 0)).contains(&idx)
        || (isolated_pawn_mg_weight_idx(0)..isolated_pawn_eg_weight_idx(0)).contains(&idx)
        || (doubled_pawn_mg_weight_idx(0)..doubled_pawn_eg_weight_idx(0)).contains(&idx)
}
