use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{HearsayByte, InstinctByte, KappaByte, KappaDetail16};
use insa_types::{DictionaryDigest, FieldMask, ObjectRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlackboardSlot {
    pub id: u32,
    pub object: ObjectRef,
    pub source_digest: DictionaryDigest,
    pub mask: FieldMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FusionRule {
    pub required: FieldMask,
    pub conflict: FieldMask,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FusionResult {
    pub status: CollapseStatus,
    pub agreed: FieldMask,
    pub conflicted: FieldMask,
    pub emits: InstinctByte,
    pub hearsay: HearsayByte,
}

pub struct FuseHearsay {
    pub slots: &'static [BlackboardSlot],
    pub rules: &'static [FusionRule],
}

impl FuseHearsay {
    pub fn fuse(&self, _ctx: &ClosureCtx) -> FusionResult {
        let mut agreed = 0;
        let mut conflicted = 0;

        for i in 0..self.slots.len() {
            agreed |= self.slots[i].mask.0;
        }

        let mut combined_emits = InstinctByte::empty();
        let mut hearsay = HearsayByte::empty();
        let mut status = CollapseStatus::Failed;

        for rule in self.rules {
            if (agreed & rule.conflict.0) == rule.conflict.0 {
                conflicted |= rule.conflict.0;
                combined_emits = combined_emits.union(rule.emits);
                hearsay = hearsay.union(HearsayByte::SOURCE_CONFLICTS);
                status = CollapseStatus::Partial;
            } else if (agreed & rule.required.0) == rule.required.0 {
                combined_emits = combined_emits.union(rule.emits);
                hearsay = hearsay.union(HearsayByte::SOURCE_AGREES);
                status = CollapseStatus::Success;
            }
        }

        FusionResult {
            status,
            agreed: FieldMask(agreed),
            conflicted: FieldMask(conflicted),
            emits: combined_emits,
            hearsay,
        }
    }
}

impl CollapseEngine for FuseHearsay {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.fuse(ctx);
        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::FUSE;
        detail.hearsay = res.hearsay;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(res.agreed),
            status: res.status,
        }
    }
}
