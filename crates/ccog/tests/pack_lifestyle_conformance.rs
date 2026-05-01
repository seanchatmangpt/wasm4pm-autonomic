//! Lifestyle pack conformance tests (Phase 12).
//!
//! Pattern: positive (bias fires) + negative (bias does not over-fire) +
//! boundary (no PII, no new variants).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::AutonomicInstinct;
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::lifestyle::{select_instinct, LifestyleBit, LifestylePack, BUILTINS};
use ccog::packs::{FieldPack, TierMasks};
use ccog::runtime::ClosedFieldContext;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("t");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

#[test]
fn pack_lifestyle_positive_fatigued_refuse_becomes_ask() {
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: (1u64 << PostureBit::ALERT) | (1u64 << LifestyleBit::FATIGUED),
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 0,
        risk_mask: 1u64 << ContextBit::THEFT_RISK,
        affordance_mask: 0,
    };
    let context = ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ctx,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };
    assert_eq!(select_instinct(&context), AutonomicInstinct::Ask);
}

#[test]
fn pack_lifestyle_negative_unfatigued_keeps_refuse() {
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
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ctx,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };
    assert_eq!(select_instinct(&context), AutonomicInstinct::Refuse);
}

#[test]
fn pack_lifestyle_boundary_overstim_also_clamps() {
    let snap = empty_snap();
    let posture = PostureBundle {
        posture_mask: (1u64 << PostureBit::ALERT) | (1u64 << LifestyleBit::OVERSTIMULATED),
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 0,
        risk_mask: 1u64 << ContextBit::THEFT_RISK,
        affordance_mask: 0,
    };
    let context = ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture,
        context: ctx,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };
    assert_eq!(select_instinct(&context), AutonomicInstinct::Ask);
}

#[test]
fn pack_lifestyle_boundary_does_not_introduce_new_variants() {
    // Sweep: every (posture, ctx) combination this pack produces must be one of
    // the canonical AutonomicInstinct variants. The match below is exhaustive —
    // adding a new variant would be a compile error.
    let snap = empty_snap();
    let inputs = [
        (1u64 << PostureBit::SETTLED, 0u64, 0u64, 0u64),
        (
            1u64 << LifestyleBit::FATIGUED,
            0,
            0,
            1u64 << ContextBit::THEFT_RISK,
        ),
        (
            1u64 << PostureBit::CADENCE_DELIVERY,
            1u64 << ContextBit::PACKAGE_EXPECTED,
            0,
            1u64 << ContextBit::CAN_RETRIEVE_NOW,
        ),
    ];
    for (pm, em, rm, am) in inputs {
        let p = PostureBundle {
            posture_mask: pm,
            confidence: 200,
        };
        let c = ContextBundle {
            expectation_mask: em,
            risk_mask: rm,
            affordance_mask: am,
        };
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: p,
            context: c,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        let v = select_instinct(&context);
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
fn pack_lifestyle_builtins_match_pack_table() {
    let from_trait = LifestylePack::builtins();
    assert_eq!(from_trait.len(), BUILTINS.len());
    assert!(
        BUILTINS.len() >= 4 && BUILTINS.len() <= 6,
        "Lifestyle pack must have 4–6 slots"
    );
}
