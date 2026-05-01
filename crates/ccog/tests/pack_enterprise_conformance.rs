//! Enterprise pack conformance tests (Phase 12).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::enterprise::{select_instinct, EnterprisePack, BUILTINS};
use ccog::packs::{FieldPack, TierMasks};
use ccog::runtime::ClosedFieldContext;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_enterprise_positive_emits_was_informed_by_and_used() {
    let snap = empty_snap();
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    let h_informed = format!(
        "{:04x}",
        ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#wasInformedBy".as_bytes()) as u16
    );
    let h_used = format!(
        "{:04x}",
        ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#used".as_bytes()) as u16
    );
    for slot in BUILTINS {
        let delta = (slot.act)(&context).expect("act");
        let nt = delta.to_ntriples();
        assert!(
            nt.contains(&h_informed),
            "enterprise slot {} missing prov:wasInformedBy",
            slot.name
        );
        assert!(
            nt.contains(&h_used),
            "enterprise slot {} missing prov:used",
            slot.name
        );
    }
}

#[test]
fn pack_enterprise_negative_no_example_org_iris() {
    let snap = empty_snap();
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    for slot in BUILTINS {
        let delta = (slot.act)(&context).expect("act");
        let nt = delta.to_ntriples();
        assert!(
            !nt.contains("example.org") && !nt.contains("example.com"),
            "enterprise slot {} leaked example IRI: {nt}",
            slot.name
        );
    }
}

#[test]
fn pack_enterprise_boundary_response_class_canonical_only() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::PostureBit;
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::CALM,
        confidence: 200,
    };
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    let v = select_instinct(&context);
    // Must be one of canonical variants (compile-time exhaustive match).
    match v {
        AutonomicInstinct::Settle
        | AutonomicInstinct::Retrieve
        | AutonomicInstinct::Inspect
        | AutonomicInstinct::Ask
        | AutonomicInstinct::Refuse
        | AutonomicInstinct::Escalate
        | AutonomicInstinct::Ignore => {}
    }
}

#[test]
fn pack_enterprise_admits_all_canonical_breeds() {
    use ccog::verdict::Breed;
    let admitted = EnterprisePack::ADMITTED_BREEDS;
    let want = [
        Breed::Eliza,
        Breed::Mycin,
        Breed::Strips,
        Breed::Shrdlu,
        Breed::Prolog,
        Breed::Hearsay,
        Breed::Dendral,
    ];
    for b in want {
        assert!(admitted.contains(&b), "enterprise should admit {:?}", b);
    }
}
