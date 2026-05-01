use insa_instinct::{InstinctByte, ShrdluByte};
use insa_kappa8::ground_shrdlu::*;
use insa_types::FieldMask;

#[test]
fn test_shrdlu_ambiguous() {
    let rules = [vendor_grounding_rule(), employee_grounding_rule()];
    // Both contexts present -> Ambiguous
    let present = FieldMask(CONTEXT_VENDOR | CONTEXT_EMPLOYEE);

    let res = GroundShrdlu::evaluate(&rules, true, present);
    assert_eq!(res.status, GroundingStatus::Ambiguous);
    assert!(res.detail.contains(ShrdluByte::AMBIGUOUS_REFERENCE));
    assert!(res.detail.contains(ShrdluByte::GROUNDING_FAILED));
    assert!(res.emits.contains(InstinctByte::INSPECT));
    assert!(res.emits.contains(InstinctByte::ASK));
}

#[test]
fn test_shrdlu_resolved() {
    let rules = [vendor_grounding_rule(), employee_grounding_rule()];
    let present = FieldMask(CONTEXT_VENDOR);

    let res = GroundShrdlu::evaluate(&rules, true, present);
    assert_eq!(res.status, GroundingStatus::Resolved);
    assert!(res.detail.contains(ShrdluByte::SYMBOL_RESOLVED));
    assert!(res.detail.contains(ShrdluByte::OBJECT_UNIQUE));
    assert!(res.emits.contains(InstinctByte::SETTLE));
    assert_eq!(res.resolved_object.unwrap().0, 100);
}
