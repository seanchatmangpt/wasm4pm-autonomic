use insa_instinct::{InstinctByte, KappaByte, StripsByte};
use insa_types::FieldMask;
use crate::schema::{ActionSchema, PolicyEpoch};
use crate::result::PreconditionResult;

/// Standard Template for INSA Kappa-8 Breeds
/// 
/// This encapsulates the byte-width logic to evaluate a cognitive schema
/// directly into InstinctByte and StripsByte representations without allocating
/// external memory on the hot path.
pub struct KappaBreedTemplate;

impl KappaBreedTemplate {
    #[inline(always)]
    pub fn evaluate(
        schema: &ActionSchema,
        present: FieldMask,
        current_epoch: PolicyEpoch,
    ) -> PreconditionResult {
        // Bitwise extraction of states
        let missing = (present.0 & schema.required.0) ^ schema.required.0;
        let forbidden = present.0 & schema.forbidden.0;

        let mut detail = StripsByte::empty();
        let mut emits = InstinctByte::empty();

        let is_stale = schema.policy_epoch.0 != current_epoch.0;

        if is_stale {
            emits = emits.union(InstinctByte::AWAIT).union(InstinctByte::ESCALATE);
            detail = detail.union(StripsByte::ACTION_BLOCKED);
        } else {
            if missing != 0 {
                detail = detail.union(StripsByte::MISSING_REQUIRED);
                emits = emits
                    .union(InstinctByte::RETRIEVE)
                    .union(InstinctByte::ASK)
                    .union(InstinctByte::AWAIT);
            }
            if forbidden != 0 {
                detail = detail.union(StripsByte::FORBIDDEN_PRESENT);
                emits = emits.union(InstinctByte::REFUSE);
            }

            let effects_conflict = (schema.add_effects.0 & schema.clear_effects.0) != 0;
            if effects_conflict {
                detail = detail.union(StripsByte::EFFECTS_CONFLICT);
                emits = emits.union(InstinctByte::INSPECT);
            } else {
                detail = detail.union(StripsByte::EFFECTS_KNOWN);
            }

            let satisfied = missing == 0 && forbidden == 0 && !effects_conflict;
            if satisfied {
                detail = detail
                    .union(StripsByte::PRECONDITIONS_SATISFIED)
                    .union(StripsByte::ACTION_ENABLED);
            } else {
                detail = detail
                    .union(StripsByte::ACTION_BLOCKED)
                    .union(StripsByte::REQUIRES_REPLAN);
                emits = emits
                    .union(InstinctByte::REFUSE)
                    .union(InstinctByte::ESCALATE);
            }
        }

        PreconditionResult {
            detail,
            kappa: KappaByte::PRECONDITION, // Replace with appropriate breed enum
            emits,
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            add_effects: schema.add_effects,
            clear_effects: schema.clear_effects,
        }
    }
}
