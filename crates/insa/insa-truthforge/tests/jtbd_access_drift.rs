use insa_hotpath::cog8::execute_cog8_graph;
use insa_instinct::InstinctByte;
use insa_security::*;
use insa_types::FieldMask;
use insa_types::Powl8Op;

#[test]
fn test_access_drift_jtbd() {
    let rows = build_access_drift_rows();

    // Given: terminated contractor + active badge/VPN/repo + vendor expired + site/device activity
    let o_star_present = FieldMask::empty()
        .with_bit(IDENTITY_TERMINATED)
        .with_bit(BADGE_ACTIVE)
        .with_bit(VPN_ACTIVE)
        .with_bit(REPO_ACCESS_ACTIVE)
        .with_bit(VENDOR_CONTRACT_EXPIRED)
        .with_bit(RECENT_SITE_ENTRY);

    // When: security graph closes field
    let decision = execute_cog8_graph(&rows, o_star_present.0, 0).expect("Graph execution failed");

    // Then: Refuse/Escalate selected
    assert!(decision.response.contains(InstinctByte::REFUSE));
    assert!(decision.fired_mask > 0);

    // Resolve POWL8 motion via admitted struct matching.
    // Reflexes translate into specific operators in process topologies.
    let selected_motion = if decision.response.contains(InstinctByte::REFUSE) {
        Powl8Op::Block
    } else if decision.response.contains(InstinctByte::ESCALATE) {
        Powl8Op::Silent // Usually deferred to out-of-band HITL
    } else {
        Powl8Op::Act
    };

    assert_eq!(selected_motion, Powl8Op::Block, "The selected action for REFUSE should systematically yield a Block motion under POWL8 mapping");
}
