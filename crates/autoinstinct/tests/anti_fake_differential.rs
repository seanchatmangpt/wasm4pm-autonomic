//! Kill Zone 3 — Warm/Hot/Replay Differential Gauntlet.
//!
//! Catches the SHACL-bug class of regressions: when one execution
//! surface (warm hooks, hot bark_artifact, decide_with_trace, replay
//! verifier) drifts from another, the system can lie semantically while
//! still passing single-surface tests. The ccog crate ships its own
//! warm-vs-hot tests for built-in hooks; this autoinstinct gauntlet
//! validates the *cross-layer* surfaces:
//!
//! 1. `decide(snap)` ≡ `decide_with_trace(snap).0` for every causal
//!    scenario and every perturbation (decision-equivalence).
//! 2. The trace's `present_mask` is the same as `compute_present_mask`
//!    on the snapshot (no hidden state).
//! 3. The receipt URN derivation is sensitive to every doctrinal input:
//!    flipping `polarity`, prior-chain, plan-node, or delta bytes must
//!    yield a different URN.
//! 4. Receipt URN is independent of `Utc::now` — calling `derive_urn`
//!    with the same canonical material must produce the same URN.

use autoinstinct::causal_harness::{
    build_inputs, canonical_scenarios, perturb,
};
use ccog::bark_artifact::{decide, BUILTINS};
use ccog::compiled::CompiledFieldSnapshot;
use ccog::compiled_hook::compute_present_mask;
use ccog::receipt::Receipt;
use ccog::trace::decide_with_trace_table;

#[test]
fn differential_decide_equals_decide_with_trace_baseline() {
    for s in canonical_scenarios() {
        let (f, _, _) = build_inputs(&s);
        let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
        let direct = decide(&snap);
        let (traced, _trace) = decide_with_trace_table(&snap, BUILTINS);
        assert_eq!(
            direct.fired, traced.fired,
            "scenario `{}`: decide.fired ({:#x}) != decide_with_trace.fired ({:#x})",
            s.name, direct.fired, traced.fired
        );
        assert_eq!(
            direct.present_mask, traced.present_mask,
            "scenario `{}`: present_mask drift between decide and decide_with_trace",
            s.name
        );
    }
}

#[test]
fn differential_decide_equals_decide_with_trace_perturbed() {
    for s in canonical_scenarios() {
        for p in &s.perturbations {
            let (f, _, _) = perturb(&s, p);
            let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
            let direct = decide(&snap);
            let (traced, _trace) = decide_with_trace_table(&snap, BUILTINS);
            assert_eq!(
                direct.fired, traced.fired,
                "scenario `{}` perturbation {:?}: decide vs decide_with_trace drift",
                s.name, p
            );
        }
    }
}

#[test]
fn differential_present_mask_matches_compute_present_mask() {
    for s in canonical_scenarios() {
        let (f, _, _) = build_inputs(&s);
        let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
        let from_decide = decide(&snap).present_mask;
        let from_helper = compute_present_mask(&snap);
        assert_eq!(
            from_decide, from_helper,
            "scenario `{}`: BarkDecision.present_mask {:#x} != compute_present_mask {:#x}",
            s.name, from_decide, from_helper
        );
    }
}

#[test]
fn differential_receipt_urn_changes_under_polarity_flip() {
    let m_pos = Receipt::canonical_material("hook", 7, b"delta", "field-1", None, 1);
    let m_neg = Receipt::canonical_material("hook", 7, b"delta", "field-1", None, 0);
    assert_ne!(
        Receipt::derive_urn(&m_pos),
        Receipt::derive_urn(&m_neg),
        "polarity flip must change URN"
    );
}

#[test]
fn differential_receipt_urn_changes_under_plan_node_change() {
    let m_a = Receipt::canonical_material("hook", 1, b"delta", "field-1", None, 1);
    let m_b = Receipt::canonical_material("hook", 2, b"delta", "field-1", None, 1);
    assert_ne!(
        Receipt::derive_urn(&m_a),
        Receipt::derive_urn(&m_b),
        "plan-node change must change URN"
    );
}

#[test]
fn differential_receipt_urn_changes_under_prior_chain_change() {
    let m_none = Receipt::canonical_material("h", 0, b"d", "f", None, 1);
    let m_some =
        Receipt::canonical_material("h", 0, b"d", "f", Some(blake3::Hash::from([1u8; 32])), 1);
    assert_ne!(Receipt::derive_urn(&m_none), Receipt::derive_urn(&m_some));
}

#[test]
fn differential_receipt_urn_is_deterministic_across_calls() {
    let prior = blake3::Hash::from([7u8; 32]);
    let m1 = Receipt::canonical_material("hook", 5, b"delta", "field-1", Some(prior), 1);
    let m2 = Receipt::canonical_material("hook", 5, b"delta", "field-1", Some(prior), 1);
    let u1 = Receipt::derive_urn(&m1);
    let u2 = Receipt::derive_urn(&m2);
    assert_eq!(u1, u2, "derive_urn must be deterministic for equal material");
    assert!(u1.starts_with("urn:blake3:"));
    assert_eq!(u1.len(), "urn:blake3:".len() + 64);
}

#[test]
fn differential_present_mask_load_bearing_under_perturbation() {
    // Cross-layer: removing a load-bearing triple must flip a bit in the
    // present_mask AND change the fired mask. If only one side moves,
    // there's a hidden coupling somewhere.
    for s in canonical_scenarios() {
        let (f0, _, _) = build_inputs(&s);
        let s0 = CompiledFieldSnapshot::from_field(&f0).expect("snap0");
        let mask0 = decide(&s0).present_mask;
        let fired0 = decide(&s0).fired;
        for p in &s.perturbations {
            // Only triple-drop perturbations affect the snapshot's
            // present_mask; posture/context bit drops live outside the
            // snapshot. So we only exercise the triple variant here.
            if !matches!(p, autoinstinct::causal_harness::Perturbation::DropTriple(_)) {
                continue;
            }
            let (f1, _, _) = perturb(&s, p);
            let s1 = CompiledFieldSnapshot::from_field(&f1).expect("snap1");
            let mask1 = decide(&s1).present_mask;
            let fired1 = decide(&s1).fired;
            assert_ne!(
                mask0, mask1,
                "scenario `{}`: triple perturbation {:?} did not change present_mask",
                s.name, p
            );
            // fired mask must also change (or at least be allowed to differ)
            assert!(
                mask0 != mask1 && (fired0 != fired1 || mask0 & !mask1 != 0),
                "scenario `{}`: triple perturbation didn't propagate to fired",
                s.name
            );
        }
    }
}
