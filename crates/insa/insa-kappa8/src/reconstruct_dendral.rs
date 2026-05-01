use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{DendralByte, InstinctByte, KappaByte, KappaDetail16};
use insa_types::{DictionaryDigest, FieldMask, ObjectRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fragment {
    pub id: u32,
    pub object: ObjectRef,
    pub digest: DictionaryDigest,
    pub mask: FieldMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconstructionCandidate {
    pub id: u32,
    pub support: FieldMask,
    pub satisfied: u64,
    pub score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructStatus {
    Success,
    Ambiguous,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconstructionResult {
    pub status: ReconstructStatus,
    pub selected: Option<u32>,
    pub emits: InstinctByte,
    pub dendral: DendralByte,
}

pub struct ReconstructDendral {
    pub fragments: &'static [Fragment],
    pub required_mask: FieldMask,
}

impl ReconstructDendral {
    pub fn reconstruct(&self, _ctx: &ClosureCtx) -> ReconstructionResult {
        let mut combined_mask = 0;
        for frag in self.fragments {
            combined_mask |= frag.mask.0;
        }

        let missing = (combined_mask & self.required_mask.0) ^ self.required_mask.0;

        if missing == 0 {
            ReconstructionResult {
                status: ReconstructStatus::Success,
                selected: Some(1),
                emits: InstinctByte::SETTLE,
                dendral: DendralByte::empty().union(DendralByte::FRAGMENTS_SUFFICIENT),
            }
        } else {
            ReconstructionResult {
                status: ReconstructStatus::Failed,
                selected: None,
                emits: InstinctByte::RETRIEVE.union(InstinctByte::ASK),
                dendral: DendralByte::empty().union(DendralByte::MISSING_FRAGMENT),
            }
        }
    }
}

impl CollapseEngine for ReconstructDendral {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.reconstruct(ctx);
        let status = match res.status {
            ReconstructStatus::Success => CollapseStatus::Success,
            ReconstructStatus::Ambiguous => CollapseStatus::Partial,
            ReconstructStatus::Failed => CollapseStatus::Failed,
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::RECONSTRUCT;
        detail.dendral = res.dendral;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(ctx.present),
            status,
        }
    }
}
