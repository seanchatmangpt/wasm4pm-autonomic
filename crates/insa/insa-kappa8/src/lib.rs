#![no_std]

pub mod reflect_eliza;
pub mod precondition_strips;
pub mod ground_shrdlu;
pub mod prove_prolog;
pub mod rule_mycin;
pub mod reconstruct_dendral;
pub mod fuse_hearsay;
pub mod reduce_gap_gps;

use insa_types::{FieldMask, CompletedMask, ObjectRef, PolicyEpoch, DictionaryDigest};
use insa_instinct::{KappaByte, InstinctByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cog8Support {
    pub support: FieldMask,
}

impl Cog8Support {
    pub fn new(support: FieldMask) -> Self { Self { support } }
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
    pub kappa: KappaByte,
    pub instincts: InstinctByte,
    pub support: Cog8Support,
    pub status: CollapseStatus,
}

pub trait CollapseEngine {
    const KAPPA_BIT: KappaByte;
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult;
}
