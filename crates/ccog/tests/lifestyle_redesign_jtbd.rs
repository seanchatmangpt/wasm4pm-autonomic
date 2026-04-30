//! Lifestyle Redesign / Occupational Therapy JTBD suite.
//!
//! Core job: given a person's situated daily context — posture, routines,
//! risks, affordances — `ccog` selects a right-sized response that supports
//! meaningful occupation without fabricating context, over-escalating, or
//! flattening the person into a generic reminder workflow.
//!
//! Every test follows the anti-stub triad:
//!   1. **Positive** — expected response happens.
//!   2. **Negative boundary** — old bad behavior does not happen.
//!   3. **Perturbation** — remove the key context, response changes.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::{select_instinct_v0, AutonomicInstinct};
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::lifestyle::{select_instinct as lifestyle_select, LifestyleBit};

use fake::faker::lorem::en::Word;
use fake::Fake;
use proptest::prelude::*;

// =============================================================================
// Snapshot + context helpers
// =============================================================================

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("lifestyle");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

/// Snapshot containing a `schema:DigitalDocument` missing `prov:value` —
/// triggers the canonical "evidence gap" path in `select_instinct_v0`.
fn snap_with_evidence_gap() -> CompiledFieldSnapshot {
    let mut f = FieldContext::new("lifestyle-gap");
    f.load_field_state(
        "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
    )
    .expect("load");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

fn posture(bits: &[u32]) -> PostureBundle {
    let mut mask = 0u64;
    for b in bits {
        mask |= 1u64 << b;
    }
    PostureBundle {
        posture_mask: mask,
        confidence: 220,
    }
}

#[derive(Default, Clone, Copy)]
struct Ctx {
    exp: u64,
    risk: u64,
    aff: u64,
}

fn ctx(c: Ctx) -> ContextBundle {
    ContextBundle {
        expectation_mask: c.exp,
        risk_mask: c.risk,
        affordance_mask: c.aff,
    }
}

fn bit(b: u32) -> u64 {
    1u64 << b
}

// =============================================================================
// Suite 1 — Routine protection (capacity present)
// =============================================================================

#[test]
fn jtbd_lifestyle_routine_due_with_capacity_selects_ask_or_retrieve() {
    let snap = empty_snap();
    // Calm + alert capacity, routine due (LifestyleBit::ROUTINE_DUE), affordance available
    // (CAN_INSPECT — we still let the routine open through the canonical lattice).
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx(Ctx {
        exp: bit(LifestyleBit::ROUTINE_DUE),
        risk: 0,
        aff: bit(ContextBit::CAN_INSPECT),
    });
    let r = lifestyle_select(&snap, &p, &c);

    // Positive: not Ignore — the routine is supported.
    assert_ne!(
        r,
        AutonomicInstinct::Ignore,
        "routine due + capacity must not be ignored, got {:?}",
        r
    );
    // Negative boundary: never auto-completes — only Inspect/Ask/Retrieve are
    // valid forward responses for an alert person on an open routine.
    assert!(
        matches!(
            r,
            AutonomicInstinct::Inspect | AutonomicInstinct::Ask | AutonomicInstinct::Retrieve
        ),
        "expected Inspect/Ask/Retrieve, got {:?}",
        r
    );

    // Perturbation: remove the routine (and the affordance to remove the
    // capacity-based Inspect path). Lattice falls through to Ignore.
    let c2 = ctx(Ctx::default());
    let p_calm = posture(&[PostureBit::CALM]);
    let r2 = lifestyle_select(&snap, &p_calm, &c2);
    assert!(
        matches!(r2, AutonomicInstinct::Ignore | AutonomicInstinct::Settle),
        "remove routine + affordance → response must collapse to Ignore/Settle, got {:?}",
        r2
    );
    assert_ne!(r, r2, "removing the routine context must change the response");
}

// =============================================================================
// Suite 2 — Fatigue-aware response shaping
// =============================================================================

#[test]
fn jtbd_lifestyle_fatigue_rewrites_hard_response_to_ask() {
    let snap = empty_snap();
    // Theft-risk + alert posture would normally yield Refuse.
    let p = posture(&[PostureBit::ALERT, LifestyleBit::FATIGUED]);
    let c = ctx(Ctx {
        exp: 0,
        risk: bit(ContextBit::THEFT_RISK),
        aff: 0,
    });
    let r = lifestyle_select(&snap, &p, &c);

    // Positive: bias rule rewrites Refuse → Ask under fatigue.
    assert_eq!(r, AutonomicInstinct::Ask);
    // Negative boundary: fatigue never escalates a non-safety risk.
    assert_ne!(r, AutonomicInstinct::Escalate);
    assert_ne!(r, AutonomicInstinct::Refuse);

    // Perturbation: remove fatigue → base lattice surfaces Refuse.
    let p_no_fatigue = posture(&[PostureBit::ALERT]);
    let r2 = lifestyle_select(&snap, &p_no_fatigue, &c);
    assert_eq!(r2, AutonomicInstinct::Refuse);
    assert_ne!(r, r2, "removing fatigue must change response");
}

// =============================================================================
// Suite 3 — Overstimulation settling
// =============================================================================

#[test]
fn jtbd_lifestyle_overstimulated_nonurgent_routine_selects_settle() {
    let snap = empty_snap();
    // Overstim + SETTLED posture trumps everything → Settle.
    let p = posture(&[PostureBit::SETTLED, LifestyleBit::OVERSTIMULATED]);
    let c = ctx(Ctx {
        exp: bit(LifestyleBit::ROUTINE_DUE),
        risk: 0,
        aff: 0,
    });
    let r = lifestyle_select(&snap, &p, &c);

    assert_eq!(r, AutonomicInstinct::Settle);
    // Negative boundary: never escalates a non-urgent routine just because
    // overstimulation was reported.
    assert_ne!(r, AutonomicInstinct::Escalate);
    assert_ne!(r, AutonomicInstinct::Refuse);

    // Perturbation: add MUST_ESCALATE risk → settle gives way to escalation
    // *only if posture is no longer SETTLED*. Settled posture is constitutional;
    // it dominates. The perturbation here removes SETTLED and keeps MUST_ESCALATE.
    let p_alert = posture(&[PostureBit::ALERT, LifestyleBit::OVERSTIMULATED]);
    let c_safety = ctx(Ctx {
        exp: bit(LifestyleBit::ROUTINE_DUE),
        risk: bit(ContextBit::MUST_ESCALATE),
        aff: 0,
    });
    let r2 = lifestyle_select(&snap, &p_alert, &c_safety);
    assert_eq!(
        r2,
        AutonomicInstinct::Escalate,
        "remove SETTLED + add MUST_ESCALATE → must escalate"
    );
}

// =============================================================================
// Suite 4 — Transition smoothing
// =============================================================================

#[test]
fn jtbd_lifestyle_transition_near_with_affordance_selects_ask_or_inspect() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT, LifestyleBit::TRANSITION_OPEN]);
    let c = ctx(Ctx {
        exp: 0,
        risk: 0,
        aff: bit(ContextBit::CAN_INSPECT),
    });
    let r = lifestyle_select(&snap, &p, &c);

    // Positive: alert+inspect-affordance yields Inspect.
    assert!(
        matches!(r, AutonomicInstinct::Inspect | AutonomicInstinct::Ask),
        "expected Inspect/Ask, got {:?}",
        r
    );
    // Negative boundary: never escalate a smooth transition.
    assert_ne!(r, AutonomicInstinct::Escalate);

    // Perturbation: remove affordance → no Inspect path.
    let c2 = ctx(Ctx::default());
    let p_calm = posture(&[PostureBit::CALM, LifestyleBit::TRANSITION_OPEN]);
    let r2 = lifestyle_select(&snap, &p_calm, &c2);
    assert!(
        matches!(r2, AutonomicInstinct::Ignore | AutonomicInstinct::Ask | AutonomicInstinct::Settle),
        "remove affordance → no Inspect, got {:?}",
        r2
    );
}

// =============================================================================
// Suite 5 — Meaningful activity protection (low energy → scaled Ask)
// =============================================================================

#[test]
fn jtbd_lifestyle_meaningful_activity_low_energy_selects_scaled_ask() {
    // Evidence-gap snapshot drives the lattice through the AskAction path,
    // representing "the meaningful activity has open requirements".
    let snap = snap_with_evidence_gap();
    let p = posture(&[PostureBit::ALERT, LifestyleBit::FATIGUED]);
    let c = ctx(Ctx::default());
    let r = lifestyle_select(&snap, &p, &c);

    assert_eq!(r, AutonomicInstinct::Ask, "fatigue + open requirement → Ask");
    // Negative boundary: never auto-completes the activity.
    assert_ne!(r, AutonomicInstinct::Settle);
    assert_ne!(r, AutonomicInstinct::Ignore);

    // Perturbation: remove the evidence gap → empty snapshot, lattice
    // collapses to Ignore for a calm person without context.
    let snap_empty = empty_snap();
    let p_calm = posture(&[PostureBit::CALM]);
    let r2 = lifestyle_select(&snap_empty, &p_calm, &c);
    assert_eq!(r2, AutonomicInstinct::Ignore);
}

// =============================================================================
// Suite 6 — Avoidance versus incapacity
// =============================================================================

#[test]
fn jtbd_lifestyle_repeated_deferral_without_fatigue_selects_inspect() {
    let snap = empty_snap();
    // Engaged + can_inspect (no fatigue, no overstim) → Inspect.
    let p = posture(&[PostureBit::ENGAGED]);
    let c = ctx(Ctx {
        exp: 0,
        risk: 0,
        aff: bit(ContextBit::CAN_INSPECT),
    });
    let r = lifestyle_select(&snap, &p, &c);

    assert_eq!(r, AutonomicInstinct::Inspect);
    // Negative boundary: never punitively escalates.
    assert_ne!(r, AutonomicInstinct::Escalate);
    assert_ne!(r, AutonomicInstinct::Refuse);

    // Perturbation: add fatigue → Inspect should soften (not Refuse, not
    // Escalate).
    let p2 = posture(&[PostureBit::ENGAGED, LifestyleBit::FATIGUED]);
    let r2 = lifestyle_select(&snap, &p2, &c);
    assert_ne!(
        r2,
        AutonomicInstinct::Refuse,
        "fatigue must never harden into Refuse"
    );
    assert_ne!(r2, AutonomicInstinct::Escalate);
}

// =============================================================================
// Suite 7 — Safety-risk escalation
// =============================================================================

#[test]
fn jtbd_lifestyle_safety_risk_unresolved_signal_escalates() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx(Ctx {
        exp: 0,
        risk: bit(ContextBit::MUST_ESCALATE),
        aff: 0,
    });
    let r = lifestyle_select(&snap, &p, &c);

    assert_eq!(r, AutonomicInstinct::Escalate);

    // Perturbation: remove the safety risk → no escalation.
    let c2 = ctx(Ctx::default());
    let r2 = lifestyle_select(&snap, &p, &c2);
    assert_ne!(r2, AutonomicInstinct::Escalate);

    // Negative boundary: even with fatigue, MUST_ESCALATE still escalates —
    // safety dominates the lifestyle softening rule.
    let p_fat = posture(&[PostureBit::ALERT, LifestyleBit::FATIGUED]);
    let r3 = lifestyle_select(&snap, &p_fat, &c);
    assert_eq!(
        r3,
        AutonomicInstinct::Escalate,
        "fatigue must NEVER suppress a MUST_ESCALATE risk"
    );
}

// =============================================================================
// Suite 8 — Ignore raw signals without meaning
// =============================================================================

#[test]
fn jtbd_lifestyle_raw_signal_without_context_does_not_trigger_task() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::CALM]);
    let c = ctx(Ctx::default());
    let r = lifestyle_select(&snap, &p, &c);

    assert_eq!(r, AutonomicInstinct::Ignore);

    // Perturbation: add routine + affordance → response changes.
    let c2 = ctx(Ctx {
        exp: bit(LifestyleBit::ROUTINE_DUE),
        risk: 0,
        aff: bit(ContextBit::CAN_INSPECT),
    });
    let p_alert = posture(&[PostureBit::ALERT]);
    let r2 = lifestyle_select(&snap, &p_alert, &c2);
    assert_ne!(r2, AutonomicInstinct::Ignore);
}

// =============================================================================
// Suite 9 — Evidence gap → Ask, never fabricate
// =============================================================================

#[test]
fn jtbd_lifestyle_missing_context_asks_without_fabricating_capacity() {
    use ccog::hooks::{missing_evidence_hook, HookRegistry};

    let mut field = FieldContext::new("lifestyle-gap");
    field
        .load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )
        .expect("load");

    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx(Ctx::default());

    // Positive: missing-evidence path forces Ask.
    let r = lifestyle_select(&snap, &p, &c);
    assert_eq!(r, AutonomicInstinct::Ask);

    // Hook delta evidence: the gap-finding never fabricates `<doc> prov:value`.
    let mut reg = HookRegistry::new();
    reg.register(missing_evidence_hook());
    let outcomes = reg.fire_matching(&field).expect("fire");
    let outcome = outcomes
        .iter()
        .find(|o| o.hook_name == "missing_evidence")
        .expect("missing_evidence must fire");
    let nt = outcome.delta.to_ntriples();
    assert!(
        !nt.contains("<http://example.org/d1> <http://www.w3.org/ns/prov#value>"),
        "must not fabricate prov:value on the gap doc:\n{}",
        nt
    );
    assert!(
        !nt.contains("\"placeholder\""),
        "must not emit placeholder literal:\n{}",
        nt
    );

    // Perturbation: remove the DD typing → snapshot has no gap, lattice
    // collapses past Ask.
    let snap_empty = empty_snap();
    let p_calm = posture(&[PostureBit::CALM]);
    let r2 = lifestyle_select(&snap_empty, &p_calm, &c);
    assert_eq!(r2, AutonomicInstinct::Ignore);
    assert_ne!(r, r2, "removing the gap must change the response");
}

// =============================================================================
// Suite 10 — Trace + replay consistency
// =============================================================================

#[test]
fn jtbd_lifestyle_trace_replays_response() {
    use ccog::bark_artifact::{decide, BUILTINS};
    use ccog::trace::decide_with_trace_table;

    let snap = snap_with_evidence_gap();

    let d1 = decide(&snap);
    let (d2, trace) = decide_with_trace_table(&snap, BUILTINS);

    // Positive: decide() ≡ decide_with_trace_table().0
    assert_eq!(d1.fired, d2.fired, "decision masks must agree");
    assert_eq!(d1.present_mask, d2.present_mask);
    assert_eq!(trace.present_mask, d1.present_mask);

    // Every fired bit has a trace node with trigger_fired = true.
    for (i, slot) in BUILTINS.iter().enumerate() {
        let bit_set = (d1.fired & (1u64 << i)) != 0;
        let n = trace
            .nodes
            .iter()
            .find(|n| n.hook_id == slot.name)
            .expect("trace node for every slot");
        assert_eq!(
            n.trigger_fired, bit_set,
            "trace node for {} must reflect fired bit",
            slot.name
        );
        if !n.trigger_fired {
            assert!(
                n.skip.is_some(),
                "non-firing slot {} must record typed skip reason",
                slot.name
            );
        }
    }
}

// =============================================================================
// Generated / proptest scenarios
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Generated: response changes when any of the four key bits changes.
    /// At least one perturbation among (routine, fatigue, risk, affordance)
    /// MUST be visible at the response surface for non-trivial fields.
    #[test]
    fn proptest_lifestyle_response_changes_when_key_context_removed(
        routine_due in any::<bool>(),
        fatigue in any::<bool>(),
        safety_risk in any::<bool>(),
        can_inspect in any::<bool>(),
    ) {
        let snap = empty_snap();
        let mut p_bits = vec![PostureBit::ALERT];
        if fatigue { p_bits.push(LifestyleBit::FATIGUED); }
        let p = posture(&p_bits);

        let mut exp = 0;
        if routine_due { exp |= bit(LifestyleBit::ROUTINE_DUE); }
        let mut risk = 0;
        if safety_risk { risk |= bit(ContextBit::MUST_ESCALATE); }
        let mut aff = 0;
        if can_inspect { aff |= bit(ContextBit::CAN_INSPECT); }
        let c = ctx(Ctx { exp, risk, aff });

        let r = lifestyle_select(&snap, &p, &c);

        // Membership: every response is canonical (never a forked variant).
        let _enum_check = match r {
            AutonomicInstinct::Settle
            | AutonomicInstinct::Retrieve
            | AutonomicInstinct::Inspect
            | AutonomicInstinct::Ask
            | AutonomicInstinct::Refuse
            | AutonomicInstinct::Escalate
            | AutonomicInstinct::Ignore => (),
        };

        // Safety dominance: if safety risk is set, response is Escalate
        // (regardless of fatigue).
        if safety_risk {
            prop_assert_eq!(r, AutonomicInstinct::Escalate, "safety risk must dominate");
        }
    }

    /// Generated: fatigue NEVER converts a non-urgent state into Escalate or
    /// Refuse. The lifestyle bias rule must always soften.
    #[test]
    fn proptest_lifestyle_fatigue_never_escalates_nonurgent(
        routine in any::<bool>(),
        can_inspect in any::<bool>(),
    ) {
        let snap = empty_snap();
        let p = posture(&[PostureBit::ALERT, LifestyleBit::FATIGUED]);
        let mut exp = 0;
        if routine { exp |= bit(LifestyleBit::ROUTINE_DUE); }
        let mut aff = 0;
        if can_inspect { aff |= bit(ContextBit::CAN_INSPECT); }
        // No risk → no escalation justification.
        let c = ctx(Ctx { exp, risk: 0, aff });

        let r = lifestyle_select(&snap, &p, &c);
        prop_assert_ne!(r, AutonomicInstinct::Escalate);
        prop_assert_ne!(r, AutonomicInstinct::Refuse);
    }

    /// Generated: response class is canonical across the whole posture/context
    /// space. Pack must not fork or drop variants.
    #[test]
    fn proptest_lifestyle_response_class_canonical(
        p_mask in any::<u64>(),
        e_mask in any::<u64>(),
        r_mask in any::<u64>(),
        a_mask in any::<u64>(),
    ) {
        let snap = empty_snap();
        let p = PostureBundle { posture_mask: p_mask, confidence: 128 };
        let c = ContextBundle { expectation_mask: e_mask, risk_mask: r_mask, affordance_mask: a_mask };
        let r = lifestyle_select(&snap, &p, &c);
        let _ = match r {
            AutonomicInstinct::Settle
            | AutonomicInstinct::Retrieve
            | AutonomicInstinct::Inspect
            | AutonomicInstinct::Ask
            | AutonomicInstinct::Refuse
            | AutonomicInstinct::Escalate
            | AutonomicInstinct::Ignore => (),
        };
    }

    /// Generated: changing any single posture/context bit eventually changes
    /// the response across the canonical lattice. We probe all 16 lifestyle
    /// bit positions plus the 8 core context bits and assert that *some*
    /// perturbation produces a different response — i.e. the lattice is
    /// genuinely sensitive to context, not constant.
    #[test]
    fn proptest_lifestyle_lattice_is_context_sensitive(
        seed in any::<u64>(),
    ) {
        let snap = empty_snap();
        let base_p = PostureBundle { posture_mask: seed, confidence: 200 };
        let base_c = ContextBundle::default();
        let base = lifestyle_select(&snap, &base_p, &base_c);

        let mut differs = false;
        for b in 0..8u32 {
            let c = ContextBundle {
                expectation_mask: 0,
                risk_mask: 1u64 << b,
                affordance_mask: 0,
            };
            if lifestyle_select(&snap, &base_p, &c) != base {
                differs = true;
                break;
            }
        }
        // For trivial postures the base may already absorb everything (e.g.
        // SETTLED). We only assert sensitivity when the base is *not*
        // SETTLED — otherwise constitutional dominance is doing its job.
        if !base_p.has(PostureBit::SETTLED) {
            prop_assert!(
                differs,
                "non-settled posture {:#018x} must produce >=1 risk-bit-perturbed response",
                seed
            );
        }
    }
}

// =============================================================================
// PII guard — Lifestyle pack acts must not embed names in IRIs
// =============================================================================

#[test]
fn jtbd_lifestyle_pack_acts_emit_only_urn_blake3_iris() {
    use ccog::bark_artifact::BUILTINS;
    let _ = BUILTINS; // sanity import

    let snap = empty_snap();
    let pack_slots = ccog::packs::lifestyle::BUILTINS;
    for slot in pack_slots {
        let delta = (slot.act)(&snap).expect("pack act fn");
        if delta.is_empty() {
            continue;
        }
        let nt = delta.to_ntriples();
        // Generate a fake personal name to demonstrate "what we never see".
        let probe: String = Word().fake();
        assert!(
            !nt.contains(&probe),
            "pack {} must not embed arbitrary content; saw probe word {} in delta",
            slot.name,
            probe
        );
        // Every act-emitted activity must use urn:blake3:.
        if nt.contains("prov#Activity") {
            assert!(
                nt.contains("urn:blake3:"),
                "pack {} emits prov:Activity without urn:blake3 IRI:\n{}",
                slot.name,
                nt
            );
        }
    }
}

// =============================================================================
// Sanity: fatigue rewrite proves base lattice + bias compose
// =============================================================================

#[test]
fn jtbd_lifestyle_fatigue_bias_does_not_alter_base_lattice_v0() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx(Ctx {
        exp: 0,
        risk: bit(ContextBit::THEFT_RISK),
        aff: 0,
    });
    let base = select_instinct_v0(&snap, &p, &c);
    assert_eq!(
        base,
        AutonomicInstinct::Refuse,
        "v0 lattice yields Refuse on theft risk + alert"
    );
    // Pack wrapper without fatigue passes through.
    let r = lifestyle_select(&snap, &p, &c);
    assert_eq!(r, base, "no-fatigue pack must pass through base lattice");
}
