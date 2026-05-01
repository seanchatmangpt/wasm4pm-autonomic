use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FragmentId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectRef(pub u64);

#[derive(Debug, Clone)]
pub struct Fragment {
    pub id: FragmentId,
    pub object: ObjectRef,
    pub time_stamp: u64,
    pub payload_mask: FieldMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateId(pub u32);

#[derive(Debug, Clone)]
pub struct ReconstructionCandidate {
    pub id: CandidateId,
    pub support: FieldMask,
    pub constraints_satisfied: u8,
    pub score: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructStatus {
    Unique,
    Ambiguous,
    Impossible,
}

#[derive(Debug, Clone)]
pub struct ReconstructionResult {
    pub status: ReconstructStatus,
    pub selected: Option<CandidateId>,
    pub emits: InstinctByte,
}

pub struct ReconstructDendral {
    pub candidates: &'static [ReconstructionCandidate],
}

impl ReconstructDendral {
    pub fn reconstruct(&self, ctx: &ClosureCtx) -> ReconstructionResult {
        let mut best_score = -1;
        let mut best_candidate = None;
        let mut tie_count = 0;

        for candidate in self.candidates {
            if (ctx.present.0 & candidate.support.0) == candidate.support.0 {
                if candidate.score > best_score {
                    best_score = candidate.score;
                    best_candidate = Some(candidate.id);
                    tie_count = 1;
                } else if candidate.score == best_score {
                    tie_count += 1;
                }
            }
        }

        if best_candidate.is_none() {
            ReconstructionResult {
                status: ReconstructStatus::Impossible,
                selected: None,
                emits: InstinctByte::RETRIEVE.union(InstinctByte::ASK),
            }
        } else if tie_count == 1 {
            ReconstructionResult {
                status: ReconstructStatus::Unique,
                selected: best_candidate,
                emits: InstinctByte::SETTLE,
            }
        } else {
            ReconstructionResult {
                status: ReconstructStatus::Ambiguous,
                selected: None,
                emits: InstinctByte::INSPECT.union(InstinctByte::ESCALATE),
            }
        }
    }
}

impl CollapseEngine for ReconstructDendral {
    const KAPPA_BIT: KappaByte = KappaByte::RECONSTRUCT;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.reconstruct(ctx);

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: res.emits,
            support: Cog8Support::new(ctx.present),
            status: match res.status {
                ReconstructStatus::Unique => CollapseStatus::Success,
                ReconstructStatus::Ambiguous => CollapseStatus::Partial,
                ReconstructStatus::Impossible => CollapseStatus::Failed,
            },
        }
    }
}
