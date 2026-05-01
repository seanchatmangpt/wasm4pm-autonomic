use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{GpsByte, InstinctByte, KappaByte, KappaDetail16};
use insa_types::{CompletedMask, FieldMask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalState {
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub completed: CompletedMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub width: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapOperator {
    pub id: OperatorId,
    pub required_preconditions: FieldMask,
    pub resolves: FieldMask,
    pub emits: InstinctByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannerBudget(pub u8);

pub struct ReduceGapGps {
    pub goal: GoalState,
    pub operators: &'static [GapOperator],
    pub max_depth: PlannerBudget,
}

impl ReduceGapGps {
    pub fn compute_gap(current: FieldMask, goal: &GoalState) -> Gap {
        let missing = (current.0 & goal.required.0) ^ goal.required.0;
        let forbidden = current.0 & goal.forbidden.0;
        Gap {
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            width: missing.count_ones() as u8 + forbidden.count_ones() as u8,
        }
    }
}

impl CollapseEngine for ReduceGapGps {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let gap = Self::compute_gap(ctx.present, &self.goal);
        let mut gps = GpsByte::empty().union(GpsByte::GOAL_KNOWN);
        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::REDUCE_GAP;

        if gap.width == 0 {
            gps = gps.union(GpsByte::PROGRESS_MADE);
            detail.gps = gps;
            return CollapseResult {
                detail,
                instincts: InstinctByte::SETTLE,
                support: Cog8Support::new(ctx.present),
                status: CollapseStatus::Success,
            };
        }

        gps = gps.union(GpsByte::GAP_DETECTED);

        if gap.width < 4 {
            gps = gps.union(GpsByte::GAP_SMALL);
        } else {
            gps = gps.union(GpsByte::GAP_LARGE);
        }

        let mut combined_instincts = InstinctByte::empty();
        let mut best_reduction = 0;

        for op in self.operators {
            let resolves_missing = op.resolves.0 & gap.missing_required.0;
            let resolves_forbidden = op.resolves.0 & gap.present_forbidden.0;
            let reduction = resolves_missing.count_ones() + resolves_forbidden.count_ones();

            if reduction > 0 && reduction >= best_reduction {
                best_reduction = reduction;
                combined_instincts = combined_instincts.union(op.emits);
            }
        }

        let status = if best_reduction > 0 {
            gps = gps.union(GpsByte::OPERATOR_AVAILABLE);
            CollapseStatus::Partial
        } else {
            gps = gps
                .union(GpsByte::OPERATOR_BLOCKED)
                .union(GpsByte::NO_PROGRESS);
            CollapseStatus::Failed
        };

        if best_reduction == 0 {
            combined_instincts = combined_instincts.union(InstinctByte::ESCALATE);
        }

        detail.gps = gps;

        CollapseResult {
            detail,
            instincts: combined_instincts,
            support: Cog8Support::new(ctx.present),
            status,
        }
    }
}
