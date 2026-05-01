use crate::rule_mycin::result::{MycinResult, MycinStatus};
use crate::rule_mycin::rule::{ExpertRule, PolicyEpoch};
use insa_instinct::{InstinctByte, KappaByte, MycinByte};
use insa_types::FieldMask;

pub struct RuleMycin;

impl RuleMycin {
    pub fn evaluate(
        rules: &[ExpertRule],
        present: FieldMask,
        current_epoch: PolicyEpoch,
    ) -> MycinResult {
        let mut detail = MycinByte::empty();
        let mut emits = InstinctByte::empty();

        let mut highest_confidence = 0;
        let mut selected_emits = InstinctByte::empty();
        let mut matched_rules = 0;

        for rule in rules {
            if rule.epoch.0 != current_epoch.0 {
                detail = detail.union(MycinByte::POLICY_EPOCH_STALE);
                continue;
            }

            detail = detail.union(MycinByte::POLICY_EPOCH_VALID);

            let missing = (present.0 & rule.required.0) ^ rule.required.0;
            let forbidden = present.0 & rule.forbidden.0;

            if missing == 0 && forbidden == 0 {
                matched_rules += 1;
                detail = detail.union(MycinByte::RULE_MATCHED);

                if rule.confidence.0 > highest_confidence {
                    highest_confidence = rule.confidence.0;
                    selected_emits = rule.emits;
                }
            }
        }

        if matched_rules > 1 {
            detail = detail
                .union(MycinByte::RULE_CONFLICT)
                .union(MycinByte::EXPERT_REVIEW_REQUIRED);
            emits = emits
                .union(InstinctByte::INSPECT)
                .union(InstinctByte::ESCALATE);
            return MycinResult {
                status: MycinStatus::Conflict,
                detail,
                kappa: KappaByte::RULE,
                emits,
            };
        }

        if matched_rules == 1 {
            detail = detail.union(MycinByte::RULE_FIRED);
            if highest_confidence >= 80 {
                detail = detail.union(MycinByte::CONFIDENCE_HIGH);
                emits = selected_emits;
            } else {
                detail = detail
                    .union(MycinByte::CONFIDENCE_LOW)
                    .union(MycinByte::EXPERT_REVIEW_REQUIRED);
                emits = selected_emits.union(InstinctByte::INSPECT);
            }
            MycinResult {
                status: MycinStatus::Fired,
                detail,
                kappa: KappaByte::RULE,
                emits,
            }
        } else {
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            MycinResult {
                status: MycinStatus::NoMatch,
                detail,
                kappa: KappaByte::RULE,
                emits,
            }
        }
    }
}
