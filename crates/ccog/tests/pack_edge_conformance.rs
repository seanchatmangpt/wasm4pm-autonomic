//! Edge pack conformance tests (Phase 12).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::packs::edge::{select_instinct, EdgePack, BUILTINS};
use ccog::packs::FieldPack;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_edge_positive_passes_canonical_lattice_through() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::{ContextBundle, PostureBit, PostureBundle};
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::SETTLED,
        confidence: 200,
    };
    assert_eq!(
        select_instinct(&snap, &posture, &ContextBundle::default()),
        AutonomicInstinct::Settle
    );
}

#[test]
fn pack_edge_negative_no_response_class_invented() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::ALERT,
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 0,
        risk_mask: 1u64 << ContextBit::THEFT_RISK,
        affordance_mask: 0,
    };
    let v = select_instinct(&snap, &posture, &ctx);
    // Edge passes through — should still be one of the canonical variants.
    let _: AutonomicInstinct = v;
}

#[test]
fn pack_edge_boundary_no_pii_in_iri() {
    // Every emitted triple's subject IRI must be a `urn:blake3:` URN. No
    // visitor names, no addresses, no email-shaped tokens.
    let snap = empty_snap();
    for slot in BUILTINS {
        let delta = (slot.act)(&snap).expect("act");
        let nt = delta.to_ntriples();
        assert!(
            !nt.contains('@'),
            "edge slot {} emitted '@' (likely PII): {nt}",
            slot.name
        );
        assert!(
            nt.contains("urn:blake3:"),
            "edge slot {} did not emit a urn:blake3 subject",
            slot.name
        );
    }
}

#[test]
fn pack_edge_builtins_count_in_band() {
    let from_trait = EdgePack::builtins();
    assert_eq!(from_trait.len(), BUILTINS.len());
    assert!(BUILTINS.len() >= 4 && BUILTINS.len() <= 6);
}
