#![doc = include_str!("../README.md")]
#![no_std]

pub mod fuse_hearsay;
pub mod ground_shrdlu;
pub mod precondition_strips;
pub mod prove_prolog;
pub mod reconstruct_dendral;
pub mod reduce_gap_gps;
pub mod reflect_eliza;
pub mod rule_mycin;

use insa_instinct::{InstinctByte, KappaDetail16};
use insa_types::{CompletedMask, DictionaryDigest, FieldMask, ObjectRef, PolicyEpoch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cog8Support {
    pub support: FieldMask,
}

impl Cog8Support {
    pub fn new(support: FieldMask) -> Self {
        Self { support }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosureCtx {
    pub present: FieldMask,
    pub completed: CompletedMask,
    pub object: ObjectRef,
    pub policy: PolicyEpoch,
    pub dictionary: DictionaryDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseStatus {
    Success,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapseResult {
    pub detail: KappaDetail16,
    pub instincts: InstinctByte,
    pub support: Cog8Support,
    pub status: CollapseStatus,
}

pub trait CollapseEngine {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult;
}
