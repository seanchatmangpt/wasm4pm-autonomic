use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{GpsByte, InstinctByte, KappaByte, KappaDetail16};
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
    pub required_preconditions: FieldMask,
    pub resolves: FieldMask,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone)]
pub struct GapReductionResult {
    pub status: CollapseStatus,
    pub gap: Gap,
    pub selected_operator: Option<u32>,
    pub emits: InstinctByte,
    pub gps: GpsByte,
}

pub struct ReduceGapGps {
    pub goal: GoalState,
    pub operators: &'static [GapOperator],
    pub max_depth: u8,
}

impl ReduceGapGps {
    pub fn reduce(&self, ctx: &ClosureCtx) -> GapReductionResult {
        self.search(ctx.present, 0)
    }

    fn search(&self, current_state: FieldMask, depth: u8) -> GapReductionResult {
        let missing = FieldMask(self.goal.required.0 & !current_state.0);
        let forbidden = FieldMask(self.goal.forbidden.0 & current_state.0);

        let width = missing.0.count_ones() as u8 + forbidden.0.count_ones() as u8;
        let gap = Gap {
            missing_required: missing,
            present_forbidden: forbidden,
            width,
        };

        if width == 0 {
            return GapReductionResult {
                status: CollapseStatus::Success,
                gap,
                selected_operator: None,
                emits: InstinctByte::SETTLE,
                gps: GpsByte::empty().union(GpsByte::GAP_SMALL),
            };
        }

        if depth >= self.max_depth {
            return GapReductionResult {
                status: CollapseStatus::Partial,
                gap,
                selected_operator: None,
                emits: InstinctByte::ESCALATE,
                gps: GpsByte::empty().union(GpsByte::NO_PROGRESS),
            };
        }

        // Means-ends analysis: Find operator that reduces the gap
        let mut best_operator = None;
        let mut best_resolved_count = 0;

        for op in self.operators {
            // Operator must be applicable (preconditions met in current state)
            if (current_state.0 & op.required_preconditions.0) == op.required_preconditions.0 {
                let resolved = op.resolves.0 & missing.0;
                let resolved_count = resolved.count_ones();

                if resolved_count > best_resolved_count {
                    best_resolved_count = resolved_count;
                    best_operator = Some(op);
                }
            }
        }

        if let Some(op) = best_operator {
            GapReductionResult {
                status: CollapseStatus::Partial,
                gap,
                selected_operator: Some(op.id),
                emits: op.emits,
                gps: GpsByte::empty().union(GpsByte::OPERATOR_AVAILABLE),
            }
        } else {
            GapReductionResult {
                status: CollapseStatus::Failed,
                gap,
                selected_operator: None,
                emits: if !forbidden.is_empty() {
                    InstinctByte::REFUSE.union(InstinctByte::ESCALATE)
                } else {
                    InstinctByte::ASK.union(InstinctByte::ESCALATE)
                },
                gps: GpsByte::empty().union(GpsByte::OPERATOR_BLOCKED),
            }
        }
    }
}

impl CollapseEngine for ReduceGapGps {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.reduce(ctx);

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::REDUCE_GAP;
        detail.gps = res.gps;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(FieldMask(
                res.gap.missing_required.0 | res.gap.present_forbidden.0,
            )),
            status: res.status,
        }
    }
}
