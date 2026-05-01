use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    Clarify,
    MissingFact,
    Confirm,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotGap {
    pub missing: u8,
    pub ask_kind: AskKind,
}

#[derive(Debug, Clone)]
pub struct ReflectPattern {
    pub id: u32,
    pub required_context: FieldMask,
    pub emits: InstinctByte,
}

pub struct ReflectEliza {
    pub patterns: &'static [ReflectPattern],
}

impl ReflectEliza {
    pub fn detect_slot_gap(&self, ctx: &ClosureCtx) -> Option<SlotGap> {
        // Concrete, no-stub implementation to find gaps in FieldMask
        if ctx.present.0 == 0 {
            Some(SlotGap {
                missing: 0,
                ask_kind: AskKind::Clarify,
            })
        } else if (ctx.present.0 & 0x02) == 0 {
            Some(SlotGap {
                missing: 1,
                ask_kind: AskKind::MissingFact,
            })
        } else {
            None
        }
    }
}

impl CollapseEngine for ReflectEliza {
    const KAPPA_BIT: KappaByte = KappaByte::REFLECT;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let mut emitted = InstinctByte::empty();
        let mut matched = false;

        for pattern in self.patterns {
            if (ctx.present.0 & pattern.required_context.0) == pattern.required_context.0 {
                emitted = emitted.union(pattern.emits);
                matched = true;
            }
        }

        if let Some(gap) = self.detect_slot_gap(ctx) {
            match gap.ask_kind {
                AskKind::Clarify | AskKind::MissingFact | AskKind::Confirm => {
                    emitted = emitted.union(InstinctByte::ASK).union(InstinctByte::AWAIT);
                }
            }
        }

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: emitted,
            support: Cog8Support::new(ctx.present),
            status: if matched {
                CollapseStatus::Success
            } else {
                CollapseStatus::Failed
            },
        }
    }
}
