use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{ElizaByte, InstinctByte, KappaByte, KappaDetail16};
use insa_types::{FieldBit, FieldMask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReflectPattern {
    pub id: u32,
    pub required_context: FieldMask,
    pub template_id: u32,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    Clarify,
    MissingEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotGap {
    pub missing: FieldBit,
    pub ask_kind: AskKind,
}

pub struct ReflectEliza {
    pub patterns: &'static [ReflectPattern],
    pub expected_slots: FieldMask,
}

impl ReflectEliza {
    pub fn detect_slot_gap(&self, ctx: &ClosureCtx) -> Option<SlotGap> {
        let missing = (ctx.present.0 & self.expected_slots.0) ^ self.expected_slots.0;
        if missing != 0 {
            let bit = missing.trailing_zeros() as u8;
            if bit < 64 {
                return Some(SlotGap {
                    missing: FieldBit::new_unchecked(bit),
                    ask_kind: AskKind::MissingEvidence,
                });
            }
        }
        None
    }
}

impl CollapseEngine for ReflectEliza {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let mut emits = InstinctByte::empty();
        let mut status = CollapseStatus::Failed;
        let mut eliza = ElizaByte::empty();

        for pattern in self.patterns {
            if (ctx.present.0 & pattern.required_context.0) == pattern.required_context.0 {
                emits = emits.union(pattern.emits);
                status = CollapseStatus::Success;
                eliza = eliza.union(ElizaByte::MIRROR_INTENT);
            }
        }

        if let Some(_) = self.detect_slot_gap(ctx) {
            emits = emits.union(InstinctByte::ASK);
            status = CollapseStatus::Partial;
            eliza = eliza
                .union(ElizaByte::DETECT_MISSING_SLOT)
                .union(ElizaByte::ASK_CLARIFYING);
        }

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::REFLECT;
        detail.eliza = eliza;

        CollapseResult {
            detail,
            instincts: emits,
            support: Cog8Support::new(ctx.present),
            status,
        }
    }
}
