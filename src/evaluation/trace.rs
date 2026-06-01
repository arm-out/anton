use crate::board::piece::{Color, PieceType};

use super::{
    EvalValue,
    params::{
        material_eg_weight_idx, material_mg_weight_idx, psqt_eg_weight_idx, psqt_mg_weight_idx,
    },
    weights,
};

pub const FEATURE_COUNT: usize = weights::WEIGHT_COUNT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EvalTerm {
    Material(PieceType),
    Psqt(PieceType, usize),
}

impl EvalTerm {
    pub fn mg_feature_idx(self) -> usize {
        match self {
            Self::Material(ptype) => material_mg_weight_idx(ptype as usize),
            Self::Psqt(ptype, psqt_idx) => psqt_mg_weight_idx(ptype as usize, psqt_idx),
        }
    }

    pub fn eg_feature_idx(self) -> usize {
        match self {
            Self::Material(ptype) => material_eg_weight_idx(ptype as usize),
            Self::Psqt(ptype, psqt_idx) => psqt_eg_weight_idx(ptype as usize, psqt_idx),
        }
    }
}

pub trait Trace {
    fn term(&mut self, side: Color, term: EvalTerm, count: EvalValue);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTrace;

impl Trace for NoTrace {
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
