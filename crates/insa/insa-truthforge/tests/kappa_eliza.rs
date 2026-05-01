use insa_instinct::{ElizaByte, InstinctByte};
use insa_kappa8::reflect_eliza::*;
use insa_types::FieldMask;

#[test]
fn test_reflect_eliza_missing_slots() {
    let patterns = [frustrate_loan_pattern()];
    let present = FieldMask(CONTEXT_LOAN_DENIED);
    let expected = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);

    let res = ReflectEliza::evaluate(&patterns, present, expected);
    assert_eq!(res.status, ReflectStatus::Incomplete);
    assert!(res.detail.contains(ElizaByte::DETECT_MISSING_SLOT));
    assert!(res.detail.contains(ElizaByte::ASK_CLARIFYING));
    assert!(res.emits.contains(InstinctByte::ASK));
    assert!(res.emits.contains(InstinctByte::INSPECT));
}

#[test]
fn test_reflect_eliza_match() {
    let patterns = [frustrate_loan_pattern()];
    let present = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);
    let expected = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);

    let res = ReflectEliza::evaluate(&patterns, present, expected);
    assert_eq!(res.status, ReflectStatus::Matched);
    assert!(res.detail.contains(ElizaByte::DETECT_AFFECT));
    assert!(res.detail.contains(ElizaByte::SLOW_PREMATURE_ACTION));
    assert!(res.detail.contains(ElizaByte::ASK_CLARIFYING));
    assert!(res.emits.contains(InstinctByte::INSPECT));
    assert!(res.emits.contains(InstinctByte::ASK));
    assert!(res.emits.contains(InstinctByte::AWAIT));
    assert_eq!(res.selected_pattern.unwrap().0, 1);
}
