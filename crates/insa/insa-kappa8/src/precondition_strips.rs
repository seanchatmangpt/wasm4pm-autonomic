use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone)]
pub struct ActionSchema {
    pub id: u32,
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
}

#[derive(Debug, Clone)]
pub struct PreconditionResult {
    pub satisfied: bool,
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub emits: InstinctByte,
}

pub struct PreconditionStrips {
    pub schemas: &'static [ActionSchema],
}

impl PreconditionStrips {
    pub fn evaluate_schema(&self, schema: &ActionSchema, ctx: &ClosureCtx) -> PreconditionResult {
        let missing = FieldMask(schema.required.0 & !ctx.present.0);
        let forbidden = FieldMask(schema.forbidden.0 & ctx.present.0);

        let satisfied = missing.is_empty() && forbidden.is_empty();

        let emits = if satisfied {
            InstinctByte::SETTLE
        } else {
            InstinctByte::REFUSE
                .union(InstinctByte::AWAIT)
                .union(InstinctByte::RETRIEVE)
        };

        PreconditionResult {
            satisfied,
            missing_required: missing,
            present_forbidden: forbidden,
            emits,
        }
    }
}

impl CollapseEngine for PreconditionStrips {
    const KAPPA_BIT: KappaByte = KappaByte::PRECONDITION;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let mut overall_emits = InstinctByte::empty();
        let mut any_satisfied = false;

        for schema in self.schemas {
            let res = self.evaluate_schema(schema, ctx);
            overall_emits = overall_emits.union(res.emits);
            if res.satisfied {
                any_satisfied = true;
            }
        }

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: overall_emits,
            support: Cog8Support::new(ctx.present),
            status: if any_satisfied {
                CollapseStatus::Success
            } else {
                CollapseStatus::Failed
            },
        }
    }
}
