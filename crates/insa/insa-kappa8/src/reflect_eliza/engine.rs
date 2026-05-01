use crate::reflect_eliza::pattern::{AskKind, ReflectPattern};
use crate::reflect_eliza::result::{ReflectResult, ReflectStatus};
use insa_instinct::{ElizaByte, InstinctByte, KappaByte};
use insa_types::FieldMask;

pub struct ReflectEliza;

impl ReflectEliza {
    pub fn evaluate(
        patterns: &[ReflectPattern],
        present: FieldMask,
        expected_slots: FieldMask,
    ) -> ReflectResult {
        let mut detail = ElizaByte::empty();
        let mut emits = InstinctByte::empty();

        let missing_slots = (present.0 & expected_slots.0) ^ expected_slots.0;

        if missing_slots != 0 {
            detail = detail
                .union(ElizaByte::DETECT_MISSING_SLOT)
                .union(ElizaByte::ASK_CLARIFYING);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::INSPECT);
            return ReflectResult {
                status: ReflectStatus::Incomplete,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(missing_slots),
                selected_pattern: None,
            };
        }

        let mut best_pattern = None;

        for pat in patterns {
            if (present.0 & pat.required_context.0) == pat.required_context.0 {
                best_pattern = Some(pat);
                break;
            }
        }

        if let Some(pat) = best_pattern {
            detail = detail.union(pat.eliza_detail);
            emits = emits.union(pat.emits);

            if pat.ask_kind != AskKind::None {
                emits = emits.union(InstinctByte::ASK);
                detail = detail.union(ElizaByte::ASK_CLARIFYING);
            }

            ReflectResult {
                status: ReflectStatus::Matched,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(0),
                selected_pattern: Some(pat.id),
            }
        } else {
            detail = detail.union(ElizaByte::DEFER_TO_CLOSURE);
            emits = emits.union(InstinctByte::SETTLE);
            ReflectResult {
                status: ReflectStatus::NoMatch,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(0),
                selected_pattern: None,
            }
        }
    }
}
