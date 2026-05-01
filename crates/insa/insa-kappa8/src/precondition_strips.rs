use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte, KappaDetail16, StripsByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSchema {
    pub id: u32,
    pub preconditions: FieldMask,
    pub forbidden: FieldMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreconditionResult {
    pub satisfied: bool,
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub emits: InstinctByte,
    pub strips: StripsByte,
}

pub struct PreconditionStrips {
    pub schemas: &'static [ActionSchema],
}

impl PreconditionStrips {
    pub fn evaluate_schema(schema: &ActionSchema, present: FieldMask) -> PreconditionResult {
        let missing = (present.0 & schema.preconditions.0) ^ schema.preconditions.0;
        let forbidden = present.0 & schema.forbidden.0;

        let satisfied = missing == 0 && forbidden == 0;

        let mut strips = StripsByte::empty();
        if satisfied {
            strips = strips
                .union(StripsByte::PRECONDITIONS_SATISFIED)
                .union(StripsByte::ACTION_ENABLED);
        } else {
            strips = strips.union(StripsByte::ACTION_BLOCKED);
        }

        let emits = if satisfied {
            InstinctByte::empty()
        } else {
            let mut i = InstinctByte::empty();
            if missing != 0 {
                i = i.union(InstinctByte::RETRIEVE);
                strips = strips.union(StripsByte::MISSING_REQUIRED);
            }
            if forbidden != 0 {
                i = i.union(InstinctByte::REFUSE);
                strips = strips.union(StripsByte::FORBIDDEN_PRESENT);
            }
            i
        };

        PreconditionResult {
            satisfied,
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            emits,
            strips,
        }
    }
}

impl CollapseEngine for PreconditionStrips {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let mut combined_instincts = InstinctByte::empty();
        let mut combined_strips = StripsByte::empty();
        let mut any_satisfied = false;
        let mut all_failed = true;

        for schema in self.schemas {
            let res = Self::evaluate_schema(schema, ctx.present);
            combined_instincts = combined_instincts.union(res.emits);
            combined_strips = combined_strips.union(res.strips);
            if res.satisfied {
                any_satisfied = true;
                all_failed = false;
            }
        }

        let status = if any_satisfied {
            CollapseStatus::Success
        } else if all_failed {
            CollapseStatus::Failed
        } else {
            CollapseStatus::Partial
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::PRECONDITION;
        detail.strips = combined_strips;

        CollapseResult {
            detail,
            instincts: combined_instincts,
            support: Cog8Support::new(ctx.present),
            status,
        }
    }
}
