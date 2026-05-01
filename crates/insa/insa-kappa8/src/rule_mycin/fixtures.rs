use crate::rule_mycin::rule::{Confidence, ExpertRule, ExpertRuleId, PolicyEpoch};
use insa_instinct::InstinctByte;
use insa_types::FieldMask;

pub const FEVER: u64 = 1 << 0;
pub const COUGH: u64 = 1 << 1;

pub fn covid_rule() -> ExpertRule {
    ExpertRule {
        id: ExpertRuleId(1),
        required: FieldMask(FEVER | COUGH),
        forbidden: FieldMask(0),
        emits: InstinctByte::ESCALATE, // High risk
        confidence: Confidence(85),
        epoch: PolicyEpoch(1),
    }
}
