use insa_instinct::{DendralByte, InstinctByte};
use insa_kappa8::reconstruct_dendral::*;
use insa_types::FieldMask;

#[test]
fn test_dendral_reconstruct_unique() {
    let rules = [identity_reconstruction_rule()];
    let present = FieldMask(FRAG_IP | FRAG_USER_AGENT);

    let res = ReconstructDendral::evaluate(&rules, present);
    assert_eq!(res.status, DendralStatus::Unique);
    assert!(res.detail.contains(DendralByte::FRAGMENTS_SUFFICIENT));
    assert!(res.detail.contains(DendralByte::UNIQUE_RECONSTRUCTION));
    assert!(res.emits.contains(InstinctByte::SETTLE));
}

#[test]
fn test_dendral_missing_fragments() {
    let rules = [identity_reconstruction_rule()];
    let present = FieldMask(FRAG_IP); // Missing user agent

    let res = ReconstructDendral::evaluate(&rules, present);
    assert_eq!(res.status, DendralStatus::Failed);
    assert!(res.detail.contains(DendralByte::MISSING_FRAGMENT));
    assert!(res.emits.contains(InstinctByte::RETRIEVE));
}
