use crate::ground_shrdlu::result::{GroundingResult, GroundingStatus};
use crate::ground_shrdlu::symbol::GroundingRule;
use insa_instinct::{InstinctByte, KappaByte, ShrdluByte};
use insa_types::FieldMask;

pub struct GroundShrdlu;

impl GroundShrdlu {
    pub fn evaluate(
        rules: &[GroundingRule],
        symbol_detected: bool,
        present: FieldMask,
    ) -> GroundingResult {
        let mut detail = ShrdluByte::empty();
        let mut emits = InstinctByte::empty();

        if !symbol_detected {
            detail = detail.union(ShrdluByte::MISSING_OBJECT);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            return GroundingResult {
                status: GroundingStatus::Missing,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            };
        }

        let mut matched_rules = 0;
        let mut last_object = None;

        for rule in rules {
            if (present.0 & rule.required_context.0) == rule.required_context.0 {
                matched_rules += 1;
                last_object = Some(rule.expected_object);
            }
        }

        if matched_rules == 1 {
            detail = detail
                .union(ShrdluByte::SYMBOL_RESOLVED)
                .union(ShrdluByte::OBJECT_UNIQUE);
            emits = emits.union(InstinctByte::SETTLE);
            GroundingResult {
                status: GroundingStatus::Resolved,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: last_object,
            }
        } else if matched_rules > 1 {
            detail = detail
                .union(ShrdluByte::AMBIGUOUS_REFERENCE)
                .union(ShrdluByte::GROUNDING_FAILED);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ASK);
            GroundingResult {
                status: GroundingStatus::Ambiguous,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            }
        } else {
            detail = detail.union(ShrdluByte::GROUNDING_FAILED);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            GroundingResult {
                status: GroundingStatus::Failed,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            }
        }
    }
}
