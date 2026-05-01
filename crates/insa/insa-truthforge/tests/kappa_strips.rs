use insa_instinct::{InstinctByte, StripsByte};
use insa_kappa8::precondition_strips::engine::PreconditionStrips;
use insa_kappa8::precondition_strips::fixtures::*;
use insa_kappa8::precondition_strips::schema::PolicyEpoch;
use insa_types::FieldMask;

#[test]
fn test_allow_access_drift_rejected() {
    let schema = allow_access_schema();

    // Field has identity terminated, vendor expired, but badge active
    let present = FieldMask(IDENTITY_TERMINATED | VENDOR_EXPIRED | BADGE_ACTIVE);

    let res = PreconditionStrips::evaluate(&schema, present, PolicyEpoch(1));

    // Should be missing requirements
    assert!(res.detail.contains(StripsByte::MISSING_REQUIRED));
    assert!(res.detail.contains(StripsByte::FORBIDDEN_PRESENT));
    assert!(res.detail.contains(StripsByte::ACTION_BLOCKED));
    assert!(res.detail.contains(StripsByte::REQUIRES_REPLAN));

    // Emits Refuse + Retrieve + Escalate + Ask + Await
    assert!(res.emits.contains(InstinctByte::REFUSE));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}

#[test]
fn test_revoke_access_enabled() {
    let schema = revoke_access_schema();

    let present = FieldMask(ACTIVE_ACCESS_PRESENT | REVOCATION_AUTHORITY_PRESENT);

    let res = PreconditionStrips::evaluate(&schema, present, PolicyEpoch(1));

    // Action should be enabled
    assert!(res.detail.contains(StripsByte::PRECONDITIONS_SATISFIED));
    assert!(res.detail.contains(StripsByte::ACTION_ENABLED));
    assert!(!res.detail.contains(StripsByte::ACTION_BLOCKED));
}
