use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertaintyLane(pub i8);

#[derive(Debug, Clone)]
pub struct ExpertRule {
    pub id: u32,
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub certainty: CertaintyLane,
}

#[derive(Debug, Clone)]
pub struct RuleClosureResult {
    pub fired: u32,
    pub emits: InstinctByte,
    pub kappa: KappaByte,
    pub support: FieldMask,
}

pub struct RuleMycin {
    pub rules: &'static [ExpertRule],
}

impl RuleMycin {
    pub fn evaluate_rules(&self, ctx: &ClosureCtx) -> Option<RuleClosureResult> {
        let mut best_rule: Option<&ExpertRule> = None;
        let mut max_certainty = CertaintyLane(-128);

        for rule in self.rules {
            let req_met = (ctx.present.0 & rule.required.0) == rule.required.0;
            let forb_met = (ctx.present.0 & rule.forbidden.0) == 0;

            if req_met && forb_met && rule.certainty.0 > max_certainty.0 {
                max_certainty = rule.certainty;
                best_rule = Some(rule);
            }
        }

        best_rule.map(|rule| RuleClosureResult {
            fired: rule.id,
            emits: rule.emits,
            kappa: rule.kappa,
            support: rule.required,
        })
    }
}

impl CollapseEngine for RuleMycin {
    const KAPPA_BIT: KappaByte = KappaByte::RULE;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        if let Some(res) = self.evaluate_rules(ctx) {
            CollapseResult {
                kappa: Self::KAPPA_BIT.union(res.kappa),
                instincts: res.emits,
                support: Cog8Support::new(res.support),
                status: CollapseStatus::Success,
            }
        } else {
            CollapseResult {
                kappa: Self::KAPPA_BIT,
                instincts: InstinctByte::INSPECT.union(InstinctByte::AWAIT),
                support: Cog8Support::new(ctx.present),
                status: CollapseStatus::Failed,
            }
        }
    }
}
