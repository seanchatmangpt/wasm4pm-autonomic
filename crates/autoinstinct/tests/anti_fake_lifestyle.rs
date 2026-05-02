//! Phase 7.1 — Lifestyle Overlap Gauntlet (KZ9).
//!
//! Proves the universal Lifestyle Redesign engine collapses overlapping
//! cognition fields (Routine ⊕ Capacity ⊕ Safety ⊕ Evidence ⊕ Meaning)
//! into the canonical 7-class lattice without forking it.
//!
//! Constitutional invariant: packs bias the lattice; they never fork
//! it. "Smallest version" / "scale meaningful activity" semantics live
//! in `matched_rule_id` and `matched_group_id`, not in a new response
//! class. This test file does not import `Soften`, `Scale`, `Reframe`,
//! `Coach`, `Delay`, `Motivate` — those are rendering hints, not
//! response classes.
//!
//! Master narrative (`master_lifestyle_overlap_collapses_to_canonical_lattice`):
//! same fatigue signal softens routine to `Ask` AND escalates driving
//! to `Refuse` — *because precedence is observable in matched_group_id*.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::{select_instinct_v0, AutonomicInstinct};
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::{
    select_instinct_with_pack_tiered, sort_groups_by_precedence, validate, TierMasks,
};
use ccog::packs::lifestyle_overlap::{
    build_lifestyle_overlap_pack, CapacityBit, EvidenceBit, MeaningBit, RoutineBit, SafetyBit,
    PRECEDENCE_CAPACITY, PRECEDENCE_EVIDENCE, PRECEDENCE_MEANING, PRECEDENCE_ROUTINE,
    PRECEDENCE_SAFETY,
};

// ============================================================================
// Fixtures
// ============================================================================

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("phase71_lifestyle");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

fn pack() -> ccog::packs::LoadedFieldPack {
    let mut p = build_lifestyle_overlap_pack("test.lifestyle.overlap", "urn:blake3:fixture");
    sort_groups_by_precedence(&mut p);
    p
}

// ============================================================================
// Field-level tests
// ============================================================================

#[test]
fn lifestyle_pack_validates_clean() {
    let p = pack();
    validate(&p).expect("Phase 7.1 lifestyle pack must validate");
}

#[test]
fn lifestyle_fatigue_softens_routine_to_ask() {
    // Routine due + fatigue high (no safety/evidence/meaning) collapses
    // to Ask via lifestyle.capacity.fatigue_softens_routine, NOT to
    // Inspect (which is the routine.missed branch) and NOT to Refuse.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let tiers = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE) | (1u64 << CapacityBit::FATIGUE_HIGH),
        k2: 0,
        k3: 0,
    };
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: tiers.clone(), human_burden: 0 }, &pack());
    assert_eq!(d.response, AutonomicInstinct::Ask);
    assert_eq!(d.matched_group_id.map(|g| g.0), Some("lifestyle.capacity"));
    assert_eq!(
        d.matched_rule_id.map(|r| r.0),
        Some("lifestyle.capacity.fatigue_softens_routine")
    );
}

#[test]
fn lifestyle_safety_overrides_capacity_for_driving() {
    // The same fatigue signal that softens chores must NOT soften
    // driving. The driving-risk K1 bit should fire `Refuse` via
    // lifestyle.safety, regardless of fatigue + routine pressure.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let tiers = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE)
            | (1u64 << CapacityBit::FATIGUE_HIGH)
            | (1u64 << SafetyBit::DRIVING_RISK),
        k2: 0,
        k3: 0,
    };
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: tiers.clone(), human_burden: 0 }, &pack());
    assert_eq!(d.response, AutonomicInstinct::Refuse);
    assert_eq!(d.matched_group_id.map(|g| g.0), Some("lifestyle.safety"));
}

#[test]
fn lifestyle_evidence_gap_asks_not_fabricates() {
    // Meal evidence missing: the system must Ask, not fabricate
    // closure. Even with routine + capacity context, evidence outranks.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let tiers = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE) | (1u64 << CapacityBit::FATIGUE_HIGH),
        k2: 0,
        k3: 1u64 << EvidenceBit::MEAL_EVIDENCE_MISSING,
    };
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: tiers.clone(), human_burden: 0 }, &pack());
    assert_eq!(d.response, AutonomicInstinct::Ask);
    assert_eq!(d.matched_group_id.map(|g| g.0), Some("lifestyle.evidence"));
    assert_eq!(
        d.matched_rule_id.map(|r| r.0),
        Some("lifestyle.evidence.missing_completion_asks")
    );
}

#[test]
fn lifestyle_meaning_scales_activity_without_new_response_class() {
    // Identity-reinforcing activity available -> Retrieve (smaller
    // version), NOT a new "Scale" or "Reframe" response. The "scale"
    // semantic lives in matched_rule_id.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let tiers = TierMasks {
        k1: 0,
        k2: 1u64 << MeaningBit::IDENTITY_REINFORCING_AVAILABLE,
        k3: 0,
    };
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: tiers.clone(), human_burden: 0 }, &pack());
    // Response is canonical Retrieve — never an enum we forked.
    assert_eq!(d.response, AutonomicInstinct::Retrieve);
    assert_eq!(d.matched_group_id.map(|g| g.0), Some("lifestyle.meaning"));
    assert_eq!(
        d.matched_rule_id.map(|r| r.0),
        Some("lifestyle.meaning.scale_meaningful_activity")
    );
    // Constitutional check: response must be one of the canonical 7.
    assert!(matches!(
        d.response,
        AutonomicInstinct::Settle
            | AutonomicInstinct::Retrieve
            | AutonomicInstinct::Inspect
            | AutonomicInstinct::Ask
            | AutonomicInstinct::Refuse
            | AutonomicInstinct::Escalate
            | AutonomicInstinct::Ignore
    ));
}

// ============================================================================
// Perturbation tests — overlap is load-bearing
// ============================================================================

#[test]
fn lifestyle_drop_capacity_bit_changes_routine_response() {
    // With routine + fatigue, response = Ask via capacity group.
    // Drop the fatigue bit; the capacity rule no longer fires; the
    // routine group fires `Ask` via routine.due_asks instead — same
    // canonical class but a different group_id, proving the matched
    // path actually depended on the capacity bit.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();

    let with_fatigue = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE) | (1u64 << CapacityBit::FATIGUE_HIGH),
        k2: 0,
        k3: 0,
    };
    let without_fatigue = TierMasks {
        k1: 1u64 << RoutineBit::ROUTINE_DUE,
        k2: 0,
        k3: 0,
    };

    let d_fatigue = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: with_fatigue.clone(), human_burden: 0 }, &pack());
    let d_none = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: without_fatigue.clone(), human_burden: 0 }, &pack());

    assert_eq!(d_fatigue.matched_group_id.map(|g| g.0), Some("lifestyle.capacity"));
    assert_eq!(d_none.matched_group_id.map(|g| g.0), Some("lifestyle.routine"));
    assert_ne!(
        d_fatigue.matched_rule_id, d_none.matched_rule_id,
        "dropping the capacity bit must change the matched rule id"
    );
}

#[test]
fn lifestyle_drop_safety_bit_changes_driving_response() {
    // Safety bit present -> Refuse via lifestyle.safety.
    // Drop the safety bit -> falls back through evidence/capacity/
    // meaning/routine; with routine+fatigue still set, the capacity
    // group wins and the response collapses to Ask.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();

    let with_safety = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE)
            | (1u64 << CapacityBit::FATIGUE_HIGH)
            | (1u64 << SafetyBit::DRIVING_RISK),
        k2: 0,
        k3: 0,
    };
    let without_safety = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE) | (1u64 << CapacityBit::FATIGUE_HIGH),
        k2: 0,
        k3: 0,
    };

    let d_safety = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: with_safety.clone(), human_burden: 0 }, &pack());
    let d_no_safety =
        select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: without_safety.clone(), human_burden: 0 }, &pack());

    assert_eq!(d_safety.response, AutonomicInstinct::Refuse);
    assert_eq!(d_safety.matched_group_id.map(|g| g.0), Some("lifestyle.safety"));
    assert_eq!(d_no_safety.response, AutonomicInstinct::Ask);
    assert_eq!(
        d_no_safety.matched_group_id.map(|g| g.0),
        Some("lifestyle.capacity")
    );
    assert_ne!(d_safety.response, d_no_safety.response);
}

#[test]
fn lifestyle_precedence_is_observable_in_matched_group_id() {
    // Construct a context where Safety, Evidence, Capacity, Meaning,
    // and Routine could all fire. Precedence_rank must determine which
    // one wins, AND the choice must be observable in matched_group_id.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();

    let all_lit = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE)
            | (1u64 << CapacityBit::FATIGUE_HIGH)
            | (1u64 << SafetyBit::DRIVING_RISK),
        k2: 1u64 << MeaningBit::IDENTITY_REINFORCING_AVAILABLE,
        k3: 1u64 << EvidenceBit::MEAL_EVIDENCE_MISSING,
    };
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: all_lit.clone(), human_burden: 0 }, &pack());

    // Safety has the lowest precedence_rank (10) and must win.
    assert_eq!(d.matched_group_id.map(|g| g.0), Some("lifestyle.safety"));
    assert_eq!(d.response, AutonomicInstinct::Refuse);

    // Sanity: the precedence ranks themselves are strictly ascending
    // in the order Safety < Evidence < Capacity < Meaning < Routine.
    assert!(PRECEDENCE_SAFETY < PRECEDENCE_EVIDENCE);
    assert!(PRECEDENCE_EVIDENCE < PRECEDENCE_CAPACITY);
    assert!(PRECEDENCE_CAPACITY < PRECEDENCE_MEANING);
    assert!(PRECEDENCE_MEANING < PRECEDENCE_ROUTINE);
}

#[test]
fn lifestyle_no_context_falls_through_to_v0_baseline() {
    // No tier bits set: pack contributes nothing and the decision
    // falls through to select_instinct_v0. This proves the pack is not
    // a blanket override.
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let d = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: TierMasks::ZERO.clone(), human_burden: 0 }, &pack());

    let baseline = select_instinct_v0(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 });
    assert_eq!(d.response, baseline);
    assert!(d.matched_group_id.is_none());
    assert!(d.matched_rule_id.is_none());
}

// ============================================================================
// Master narrative — same fatigue softens routine, escalates driving
// ============================================================================

#[test]
fn master_lifestyle_overlap_collapses_to_canonical_lattice() {
    let snap = empty_snap();
    let posture = PostureBundle::default();
    let ctx = ContextBundle::default();
    let p = pack();

    // Same fatigue signal in two scenarios:
    let chore_scenario = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE) | (1u64 << CapacityBit::FATIGUE_HIGH),
        k2: 0,
        k3: 0,
    };
    let driving_scenario = TierMasks {
        k1: (1u64 << RoutineBit::ROUTINE_DUE)
            | (1u64 << CapacityBit::FATIGUE_HIGH)
            | (1u64 << SafetyBit::DRIVING_RISK),
        k2: 0,
        k3: 0,
    };

    let chore = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: chore_scenario.clone(), human_burden: 0 }, &p);
    let driving = select_instinct_with_pack_tiered(&ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap.clone()), posture: posture.clone(), context: ctx.clone(), tiers: driving_scenario.clone(), human_burden: 0 }, &p);

    // The thesis: same fatigue, different field overlap, different
    // canonical response — and precedence is observable.
    assert_eq!(chore.response, AutonomicInstinct::Ask);
    assert_eq!(chore.matched_group_id.map(|g| g.0), Some("lifestyle.capacity"));

    assert_eq!(driving.response, AutonomicInstinct::Refuse);
    assert_eq!(driving.matched_group_id.map(|g| g.0), Some("lifestyle.safety"));

    assert_ne!(chore.response, driving.response);
    assert_ne!(chore.matched_group_id, driving.matched_group_id);

    // Constitutional: every produced response is in the canonical 7.
    for d in [&chore, &driving] {
        assert!(matches!(
            d.response,
            AutonomicInstinct::Settle
                | AutonomicInstinct::Retrieve
                | AutonomicInstinct::Inspect
                | AutonomicInstinct::Ask
                | AutonomicInstinct::Refuse
                | AutonomicInstinct::Escalate
                | AutonomicInstinct::Ignore
        ));
    }
}

#[test]
fn lifestyle_no_assert_true_placeholders_remain() {
    let src = include_str!("anti_fake_lifestyle.rs");
    let needle = format!("{}{}", "assert!(\n        true", ",");
    assert!(
        !src.contains(&needle),
        "release-blocking assert!(true) placeholder must not remain in KZ9 tests"
    );
    let needle2 = format!("{}{}", "assert!(true", ",");
    assert!(
        !src.contains(&needle2),
        "release-blocking assert!(true) placeholder must not remain in KZ9 tests"
    );
}
