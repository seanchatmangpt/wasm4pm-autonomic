//! Cross-pack namespace isolation tests (Phase 12).
//!
//! Verifies that:
//! - No two packs claim overlapping bit ranges.
//! - All four packs produce only canonical [`AutonomicInstinct`] variants
//!   (no per-pack response-class fork).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::AutonomicInstinct;
use ccog::multimodal::{ContextBundle, PostureBit, PostureBundle};
use ccog::packs::dev::DevPack;
use ccog::packs::edge::EdgePack;
use ccog::packs::enterprise::EnterprisePack;
use ccog::packs::lifestyle::LifestylePack;
use ccog::packs::FieldPack;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_namespace_isolation_no_bit_overlap() {
    let ranges = [
        ("lifestyle", LifestylePack::POSTURE_RANGE),
        ("edge", EdgePack::POSTURE_RANGE),
        ("enterprise", EnterprisePack::POSTURE_RANGE),
        ("dev", DevPack::POSTURE_RANGE),
    ];
    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (na, ra) = (ranges[i].0, ranges[i].1.clone());
            let (nb, rb) = (ranges[j].0, ranges[j].1.clone());
            assert!(
                ra.end <= rb.start || rb.end <= ra.start,
                "{} ({:?}) overlaps {} ({:?})",
                na,
                ra,
                nb,
                rb
            );
        }
    }
}

#[test]
fn pack_response_class_canonical_only() {
    // For each pack's bias wrapper, exhaustively verify the output is one of
    // the seven canonical AutonomicInstinct variants. The compile-time
    // exhaustive match below would fail to type-check if a pack ever
    // introduced a new variant.
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::ALERT,
        confidence: 200,
    };
    let ctx = ContextBundle::default();

    let outputs = [
        ccog::packs::lifestyle::select_instinct(&snap, &posture, &ctx),
        ccog::packs::edge::select_instinct(&snap, &posture, &ctx),
        ccog::packs::enterprise::select_instinct(&snap, &posture, &ctx),
        ccog::packs::dev::select_instinct(&snap, &posture, &ctx),
    ];

    for v in outputs {
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
}

#[test]
fn pack_namespace_each_pack_lands_in_its_band() {
    use ccog::packs::bits::{DEV_RANGE, EDGE_RANGE, ENTERPRISE_RANGE, LIFESTYLE_RANGE};

    assert_eq!(LifestylePack::POSTURE_RANGE, LIFESTYLE_RANGE);
    assert_eq!(EdgePack::POSTURE_RANGE, EDGE_RANGE);
    assert_eq!(EnterprisePack::POSTURE_RANGE, ENTERPRISE_RANGE);
    assert_eq!(DevPack::POSTURE_RANGE, DEV_RANGE);
}
