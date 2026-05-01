#![no_std]

pub mod fuse_hearsay;
pub mod ground_shrdlu;
pub mod precondition_strips;
pub mod prove_prolog;
pub mod reconstruct_dendral;
pub mod reduce_gap_gps;
pub mod reflect_eliza;
pub mod rule_mycin;

use insa_instinct::{InstinctByte, KappaByte};
use insa_types::{CompletedMask, FieldMask};

#[derive(Debug, Clone, Copy)]
pub struct Cog8Support {
    pub support: FieldMask,
}

impl Cog8Support {
    pub fn new(support: FieldMask) -> Self {
        Self { support }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClosureCtx {
    pub present: FieldMask,
    pub completed: CompletedMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseStatus {
    Success,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Copy)]
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
