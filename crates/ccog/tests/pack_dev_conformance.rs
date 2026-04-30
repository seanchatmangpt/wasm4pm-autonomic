//! Dev / Agent Governance pack conformance tests (Phase 12).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::AutonomicInstinct;
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::dev::{select_instinct, DevPack, BUILTINS};
use ccog::packs::FieldPack;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_dev_positive_clamps_refuse_to_ask() {
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
    assert_eq!(select_instinct(&snap, &posture, &ctx), AutonomicInstinct::Ask);
}

#[test]
fn pack_dev_negative_settle_passes_through() {
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
fn pack_dev_boundary_does_not_auto_merge() {
    // For every input that would otherwise produce Refuse or Escalate, the
    // dev pack must clamp to Ask. This is the structural anti-auto-merge
    // guarantee — dev decisions always surface for human review.
    let snap = empty_snap();
    let high_pressure = [
        // Theft risk + alert → base lattice produces Refuse.
        (
            1u64 << PostureBit::ALERT,
            0u64,
            1u64 << ContextBit::THEFT_RISK,
            0u64,
        ),
        // Must escalate → base lattice produces Escalate.
        (
            1u64 << PostureBit::ALERT,
            0,
            1u64 << ContextBit::MUST_ESCALATE,
            0,
        ),
        // Safety risk without inspect → base lattice produces Escalate.
        (
            1u64 << PostureBit::ALERT,
            0,
            1u64 << ContextBit::SAFETY_RISK,
            0,
        ),
    ];
    for (pm, em, rm, am) in high_pressure {
        let p = PostureBundle { posture_mask: pm, confidence: 200 };
        let c = ContextBundle { expectation_mask: em, risk_mask: rm, affordance_mask: am };
        let v = select_instinct(&snap, &p, &c);
        assert_eq!(
            v,
            AutonomicInstinct::Ask,
            "dev pack must clamp pressure inputs to Ask, got {:?}",
            v
        );
        // Crucially: never produces a "merge" / Settle out of pressure.
        assert_ne!(v, AutonomicInstinct::Settle);
    }
}

#[test]
fn pack_dev_acts_emit_ask_action() {
    let snap = empty_snap();
    for slot in BUILTINS {
        let delta = (slot.act)(&snap).expect("act");
        let nt = delta.to_ntriples();
        assert!(
            nt.contains("schema.org/AskAction"),
            "dev slot {} missing schema:AskAction",
            slot.name
        );
    }
}

#[test]
fn pack_dev_builtins_count_in_band() {
    let from_trait = DevPack::builtins();
    assert_eq!(from_trait.len(), BUILTINS.len());
    assert!(BUILTINS.len() >= 4 && BUILTINS.len() <= 6);
}
