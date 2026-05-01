use insa_instinct::{InstinctByte, MycinByte};
use insa_kappa8::rule_mycin::*;
use insa_types::FieldMask;

#[test]
fn test_mycin_fire_high_confidence() {
    let rules = [covid_rule()];
    let present = FieldMask(FEVER | COUGH);

    let res = RuleMycin::evaluate(&rules, present, PolicyEpoch(1));
    assert_eq!(res.status, MycinStatus::Fired);
    assert!(res.detail.contains(MycinByte::RULE_MATCHED));
    assert!(res.detail.contains(MycinByte::RULE_FIRED));
    assert!(res.detail.contains(MycinByte::CONFIDENCE_HIGH));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}

#[test]
fn test_mycin_stale_epoch() {
    let rules = [covid_rule()];
    let present = FieldMask(FEVER | COUGH);

    let res = RuleMycin::evaluate(&rules, present, PolicyEpoch(2)); // Stale rule
    assert_eq!(res.status, MycinStatus::NoMatch);
    assert!(res.detail.contains(MycinByte::POLICY_EPOCH_STALE));
    assert!(res.emits.contains(InstinctByte::ASK));
}
