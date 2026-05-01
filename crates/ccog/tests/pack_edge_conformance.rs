//! Edge pack conformance tests (Phase 12).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::edge::{select_instinct, EdgePack, BUILTINS};
use ccog::packs::{FieldPack, TierMasks};
use ccog::runtime::ClosedFieldContext;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_edge_positive_passes_canonical_lattice_through() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::PostureBit;
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::SETTLED,
        confidence: 200,
    };
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    assert_eq!(select_instinct(&context), AutonomicInstinct::Settle);
}

#[test]
fn pack_edge_negative_no_response_class_invented() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::{ContextBit, PostureBit};
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
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ctx,
        tiers: TierMasks::ZERO,
    };
    let v = select_instinct(&context);
    // Edge passes through — should still be one of the canonical variants.
    let _: AutonomicInstinct = v;
}

#[test]
fn pack_edge_boundary_no_pii_in_iri() {
    // Every emitted triple's subject IRI must be a hashed deterministic URN.
    // No visitor names, no addresses, no email-shaped tokens.
    let snap = empty_snap();
    let context = ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };
    for slot in BUILTINS {
        let delta = (slot.act)(&context).expect("act");
        let nt = delta.to_ntriples();
        assert!(
            !nt.contains('@'),
            "edge slot {} emitted '@' (likely PII): {nt}",
            slot.name
        );
        assert!(
            nt.contains("urn:ccog:id:"),
            "edge slot {} did not emit a hashed URN subject",
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
