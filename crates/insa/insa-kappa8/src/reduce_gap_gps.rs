use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::{CompletedMask, FieldMask};

#[derive(Debug, Clone)]
pub struct GoalState {
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub completed: CompletedMask,
}

#[derive(Debug, Clone)]
pub struct Gap {
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub width: u8,
}

#[derive(Debug, Clone)]
pub struct GapOperator {
    pub id: u32,
    pub resolves: FieldMask,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone)]
pub struct GapReductionResult {
    pub gap: Gap,
    pub selected_operator: Option<GapOperator>,
    pub emits: InstinctByte,
}

pub struct ReduceGapGps {
    pub goal: GoalState,
    pub operators: &'static [GapOperator],
}

impl ReduceGapGps {
    pub fn reduce(&self, ctx: &ClosureCtx) -> GapReductionResult {
        let missing = FieldMask(self.goal.required.0 & !ctx.present.0);
        let forbidden = FieldMask(self.goal.forbidden.0 & ctx.present.0);

        let width = missing.0.count_ones() as u8 + forbidden.0.count_ones() as u8;

        let gap = Gap {
            missing_required: missing,
            present_forbidden: forbidden,
            width,
        };

        if width == 0 {
            return GapReductionResult {
                gap,
                selected_operator: None,
                emits: InstinctByte::SETTLE,
            };
        }

        // GPS operator selection: pick the operator that resolves the most missing fields
        let mut best_operator = None;
        let mut best_resolved_count = 0;

        for op in self.operators {
            let resolved = op.resolves.0 & missing.0;
            let resolved_count = resolved.count_ones();
            if resolved_count > best_resolved_count {
                best_resolved_count = resolved_count;
                best_operator = Some(op.clone());
            }
        }

        let emits = if let Some(ref op) = best_operator {
            op.emits
        } else {
            // No operator can reduce the gap -> Escalate or Refuse
            if !forbidden.is_empty() {
                InstinctByte::REFUSE.union(InstinctByte::ESCALATE)
            } else {
                InstinctByte::ASK.union(InstinctByte::ESCALATE)
            }
        };

        GapReductionResult {
            gap,
            selected_operator: best_operator,
            emits,
        }
    }
}

impl CollapseEngine for ReduceGapGps {
    const KAPPA_BIT: KappaByte = KappaByte::REDUCE_GAP;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.reduce(ctx);

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: res.emits,
            support: Cog8Support::new(FieldMask(
                res.gap.missing_required.0 | res.gap.present_forbidden.0,
            )),
            status: if res.gap.width == 0 {
                CollapseStatus::Success
            } else if res.selected_operator.is_some() {
                CollapseStatus::Partial
            } else {
                CollapseStatus::Failed
            },
        }
    }
}
