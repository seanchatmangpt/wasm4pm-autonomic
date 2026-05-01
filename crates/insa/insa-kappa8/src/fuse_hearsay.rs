use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Identity,
    Physical,
    Digital,
    Policy,
    Vendor,
}

#[derive(Debug, Clone)]
pub struct BlackboardSlot {
    pub id: SlotId,
    pub object: ObjectRef,
    pub kind: EvidenceKind,
    pub source_digest: u64,
    pub freshness: u32,
    pub support: FieldMask,
}

#[derive(Debug, Clone)]
pub struct FusionRule {
    pub required: FieldMask,
    pub conflict: FieldMask,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionStatus {
    Agreed,
    Conflicted,
    Incomplete,
}

#[derive(Debug, Clone)]
pub struct FusionResult {
    pub status: FusionStatus,
    pub agreed: FieldMask,
    pub conflicted: FieldMask,
    pub emits: InstinctByte,
}

pub struct FuseHearsay {
    pub rules: &'static [FusionRule],
    pub slots: &'static [BlackboardSlot],
}

impl FuseHearsay {
    pub fn fuse(&self, ctx: &ClosureCtx) -> FusionResult {
        let mut overall_agreed = FieldMask(0);
        let mut overall_conflicted = FieldMask(0);
        let mut overall_emits = InstinctByte::empty();
        let mut matched_any = false;

        for rule in self.rules {
            if (ctx.present.0 & rule.required.0) == rule.required.0 {
                matched_any = true;
                if (ctx.present.0 & rule.conflict.0) != 0 {
                    overall_conflicted = FieldMask(overall_conflicted.0 | rule.conflict.0);
                    overall_emits =
                        overall_emits.union(InstinctByte::INSPECT.union(InstinctByte::ESCALATE));
                } else {
                    overall_agreed = FieldMask(overall_agreed.0 | rule.required.0);
                    overall_emits = overall_emits.union(rule.emits);
                }
            }
        }

        let status = if !overall_conflicted.is_empty() {
            FusionStatus::Conflicted
        } else if matched_any {
            FusionStatus::Agreed
        } else {
            FusionStatus::Incomplete
        };

        if status == FusionStatus::Incomplete {
            overall_emits = overall_emits.union(InstinctByte::RETRIEVE.union(InstinctByte::AWAIT));
        }

        FusionResult {
            status,
            agreed: overall_agreed,
            conflicted: overall_conflicted,
            emits: overall_emits,
        }
    }
}

impl CollapseEngine for FuseHearsay {
    const KAPPA_BIT: KappaByte = KappaByte::FUSE;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.fuse(ctx);

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: res.emits,
            support: Cog8Support::new(FieldMask(res.agreed.0 | res.conflicted.0)),
            status: match res.status {
                FusionStatus::Agreed => CollapseStatus::Success,
                FusionStatus::Conflicted => CollapseStatus::Failed,
                FusionStatus::Incomplete => CollapseStatus::Partial,
            },
        }
    }
}
