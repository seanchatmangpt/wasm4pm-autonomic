use crate::precondition_strips::result::PreconditionResult;
use crate::precondition_strips::schema::{ActionSchema, PolicyEpoch};
use insa_instinct::{InstinctByte, KappaByte, StripsByte};
use insa_types::FieldMask;

pub struct PreconditionStrips;

impl PreconditionStrips {
    pub fn evaluate(
        schema: &ActionSchema,
        present: FieldMask,
        current_epoch: PolicyEpoch,
    ) -> PreconditionResult {
        let missing = (present.0 & schema.required.0 .0) ^ schema.required.0 .0;
        let forbidden = present.0 & schema.forbidden.0 .0;

        let mut strips = StripsByte::empty();
        let mut emits = InstinctByte::empty();

        let is_stale = schema.policy_epoch.0 != current_epoch.0;

        if is_stale {
            emits = emits
                .union(InstinctByte::AWAIT)
                .union(InstinctByte::ESCALATE);
            strips = strips.union(StripsByte::ACTION_BLOCKED);
        } else {
            if missing != 0 {
                strips = strips.union(StripsByte::MISSING_REQUIRED);
                emits = emits
                    .union(InstinctByte::RETRIEVE)
                    .union(InstinctByte::ASK)
                    .union(InstinctByte::AWAIT);
            }
            if forbidden != 0 {
                strips = strips.union(StripsByte::FORBIDDEN_PRESENT);
                emits = emits.union(InstinctByte::REFUSE);
            }

            let effects_conflict = (schema.add_effects.0 & schema.clear_effects.0) != 0;
            if effects_conflict {
                strips = strips.union(StripsByte::EFFECTS_CONFLICT);
                emits = emits.union(InstinctByte::INSPECT);
            } else {
                strips = strips.union(StripsByte::EFFECTS_KNOWN);
            }

            let satisfied = missing == 0 && forbidden == 0 && !effects_conflict;
            if satisfied {
                strips = strips
                    .union(StripsByte::PRECONDITIONS_SATISFIED)
                    .union(StripsByte::ACTION_ENABLED);
            } else {
                strips = strips
                    .union(StripsByte::ACTION_BLOCKED)
                    .union(StripsByte::REQUIRES_REPLAN);
                emits = emits
                    .union(InstinctByte::REFUSE)
                    .union(InstinctByte::ESCALATE);
            }
        }

        PreconditionResult {
            detail: strips,
            kappa: KappaByte::PRECONDITION,
            emits,
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            add_effects: schema.add_effects,
            clear_effects: schema.clear_effects,
        }
    }
}
