use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte, KappaDetail16, MycinByte};
use insa_types::{FieldMask, RuleId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertaintyLane(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpertRule {
    pub id: RuleId,
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub emits: InstinctByte,
    pub certainty: CertaintyLane,
}

pub struct RuleClosureResult {
    pub fired: Option<RuleId>,
    pub emits: InstinctByte,
    pub mycin: MycinByte,
    pub support: FieldMask,
}

pub struct RuleMycin {
    pub rules: &'static [ExpertRule],
}

impl RuleMycin {
    pub fn evaluate_rules(&self, ctx: &ClosureCtx) -> RuleClosureResult {
        let mut best_certainty = 0;
        let mut best_rule = None;
        let mut combined_emits = InstinctByte::empty();
        let mut combined_mycin = MycinByte::empty();
        let mut combined_support = 0;

        for rule in self.rules {
            let missing = (ctx.present.0 & rule.required.0) ^ rule.required.0;
            let forbidden = ctx.present.0 & rule.forbidden.0;

            if missing == 0 && forbidden == 0 {
                combined_emits = combined_emits.union(rule.emits);
                combined_mycin = combined_mycin
                    .union(MycinByte::RULE_MATCHED)
                    .union(MycinByte::RULE_FIRED);
                combined_support |= rule.required.0;

                if rule.certainty.0 >= best_certainty {
                    best_certainty = rule.certainty.0;
                    best_rule = Some(rule.id);
                }
            }
        }

        RuleClosureResult {
            fired: best_rule,
            emits: combined_emits,
            mycin: combined_mycin,
            support: FieldMask(combined_support),
        }
    }
}

impl CollapseEngine for RuleMycin {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.evaluate_rules(ctx);

        let status = if res.fired.is_some() {
            CollapseStatus::Success
        } else {
            CollapseStatus::Failed
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::RULE;
        detail.mycin = res.mycin;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(res.support),
            status,
        }
    }
}
