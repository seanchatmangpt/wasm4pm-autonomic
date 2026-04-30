//! Reusable causal-dependency harness (Kill Zone 2 of the anti-fake gauntlet).
//!
//! A [`CausalScenario`] is a triple `(closed-context, expected response,
//! perturbations)` that proves a response class is **earned** rather than
//! returned by accident:
//!
//! - **Positive** assertion: `select_instinct_v0` returns `expected`.
//! - **Perturbation** assertion: every removal in `perturbations` either
//!   changes the response or denies it (drops to `Ignore`/`Ask`).
//!
//! Future kill zones (`anti_fake_perf`, `anti_fake_packs`, master test)
//! consume the harness so semantic gates are written once and reused.
//!
//! Hot-path purity is **not** measured here — that's Kill Zone 6's job.
//! This module only proves *what* the response is and *why* it changes.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::{select_instinct_v0, AutonomicInstinct};
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};

/// A perturbation is a closed-form mutation of the closed context.
/// Each variant removes exactly one load-bearing input.
#[derive(Clone, Debug)]
pub enum Perturbation {
    /// Remove the named N-Triple line (string match).
    DropTriple(&'static str),
    /// Clear a posture bit.
    DropPostureBit(u32),
    /// Clear an expectation context bit.
    DropExpectation(u32),
    /// Clear a risk context bit.
    DropRisk(u32),
    /// Clear an affordance context bit.
    DropAffordance(u32),
}

/// One scenario carried through positive + perturbation assertions.
pub struct CausalScenario {
    /// Stable name (for failure messages).
    pub name: &'static str,
    /// Profile token (used by future pack-aware kill zones).
    pub profile: &'static str,
    /// N-Triples loaded into the field context.
    pub field_ntriples: String,
    /// Posture bits to set (e.g. `[PostureBit::CALM]`).
    pub posture_bits: Vec<u32>,
    /// Expectation bits.
    pub expectation_bits: Vec<u32>,
    /// Risk bits.
    pub risk_bits: Vec<u32>,
    /// Affordance bits.
    pub affordance_bits: Vec<u32>,
    /// Required positive response class.
    pub expected: AutonomicInstinct,
    /// Perturbations with expected response class after removal.
    /// Each perturbation must produce this specific response (not just any different response).
    pub perturbations: Vec<(Perturbation, AutonomicInstinct)>,
}

/// Materialize a closed cognition surface from `(field, posture, ctx)`.
pub fn build_inputs(s: &CausalScenario) -> (FieldContext, PostureBundle, ContextBundle) {
    let mut field = FieldContext::new(s.name);
    if !s.field_ntriples.is_empty() {
        field
            .load_field_state(&s.field_ntriples)
            .expect("scenario field N-Triples must parse");
    }
    let mut posture = PostureBundle::default();
    for &b in &s.posture_bits {
        posture.posture_mask |= 1u64 << b;
    }
    let mut ctx = ContextBundle::default();
    for &b in &s.expectation_bits {
        ctx.expectation_mask |= 1u64 << b;
    }
    for &b in &s.risk_bits {
        ctx.risk_mask |= 1u64 << b;
    }
    for &b in &s.affordance_bits {
        ctx.affordance_mask |= 1u64 << b;
    }
    (field, posture, ctx)
}

/// Compute the response under `(field, posture, ctx)`.
pub fn respond(field: &FieldContext, posture: &PostureBundle, ctx: &ContextBundle) -> AutonomicInstinct {
    let snap = CompiledFieldSnapshot::from_field(field).expect("snapshot");
    select_instinct_v0(&snap, posture, ctx)
}

/// Apply one perturbation, returning a new closed surface.
pub fn perturb(
    s: &CausalScenario,
    p: &Perturbation,
) -> (FieldContext, PostureBundle, ContextBundle) {
    let (mut field, mut posture, mut ctx) = build_inputs(s);
    match *p {
        Perturbation::DropTriple(line) => {
            // Reload from the original N-Triples minus the offending line.
            let kept: String = s
                .field_ntriples
                .lines()
                .filter(|l| l.trim() != line.trim())
                .map(|l| format!("{l}\n"))
                .collect();
            field = FieldContext::new(s.name);
            if !kept.is_empty() {
                field.load_field_state(&kept).expect("perturbed field reloads");
            }
        }
        Perturbation::DropPostureBit(b) => {
            posture.posture_mask &= !(1u64 << b);
        }
        Perturbation::DropExpectation(b) => {
            ctx.expectation_mask &= !(1u64 << b);
        }
        Perturbation::DropRisk(b) => {
            ctx.risk_mask &= !(1u64 << b);
        }
        Perturbation::DropAffordance(b) => {
            ctx.affordance_mask &= !(1u64 << b);
        }
    }
    (field, posture, ctx)
}

/// Built-in coverage of every canonical response class.
///
/// Every entry must:
/// 1. produce its `expected` response under the given closed context, and
/// 2. produce a specific expected response when any perturbation is applied
///    (not just any different response).
#[must_use]
pub fn canonical_scenarios() -> Vec<CausalScenario> {
    use AutonomicInstinct::*;
    let dd_missing = "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n".to_string();

    vec![
        CausalScenario {
            name: "settle_via_settled_posture",
            profile: "core",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::SETTLED],
            expectation_bits: vec![],
            risk_bits: vec![],
            affordance_bits: vec![],
            expected: Settle,
            perturbations: vec![(Perturbation::DropPostureBit(PostureBit::SETTLED), Ask)],
        },
        CausalScenario {
            name: "retrieve_via_expected_package",
            profile: "edge",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::CADENCE_DELIVERY],
            expectation_bits: vec![ContextBit::PACKAGE_EXPECTED],
            risk_bits: vec![],
            affordance_bits: vec![ContextBit::CAN_RETRIEVE_NOW],
            expected: Retrieve,
            perturbations: vec![
                (Perturbation::DropExpectation(ContextBit::PACKAGE_EXPECTED), Ask),
                (Perturbation::DropAffordance(ContextBit::CAN_RETRIEVE_NOW), Ask),
                (Perturbation::DropPostureBit(PostureBit::CADENCE_DELIVERY), Ask),
            ],
        },
        CausalScenario {
            name: "inspect_via_alert_with_inspect_affordance",
            profile: "edge",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::ALERT],
            expectation_bits: vec![],
            risk_bits: vec![],
            affordance_bits: vec![ContextBit::CAN_INSPECT],
            expected: Inspect,
            perturbations: vec![
                (Perturbation::DropAffordance(ContextBit::CAN_INSPECT), Ask),
                (Perturbation::DropPostureBit(PostureBit::ALERT), Ask),
            ],
        },
        CausalScenario {
            name: "ask_via_evidence_gap",
            profile: "enterprise",
            field_ntriples: dd_missing.clone(),
            // CALM posture is structural: when DD is removed and no
            // expectations/risks remain, the lattice reaches the
            // calm-baseline branch and returns Ignore — proving Ask was
            // earned by the DD triple, not by a default fallback.
            posture_bits: vec![PostureBit::CALM],
            expectation_bits: vec![],
            risk_bits: vec![],
            affordance_bits: vec![],
            expected: Ask,
            perturbations: vec![(
                Perturbation::DropTriple(
                    "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> ."
                ),
                Ignore
            )],
        },
        CausalScenario {
            name: "refuse_via_theft_risk_alert",
            profile: "edge",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::ALERT],
            expectation_bits: vec![],
            risk_bits: vec![ContextBit::THEFT_RISK],
            affordance_bits: vec![],
            expected: Refuse,
            perturbations: vec![
                (Perturbation::DropRisk(ContextBit::THEFT_RISK), Ask),
                (Perturbation::DropPostureBit(PostureBit::ALERT), Ask),
            ],
        },
        CausalScenario {
            name: "escalate_via_must_escalate",
            profile: "enterprise",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::ALERT],
            expectation_bits: vec![],
            risk_bits: vec![ContextBit::MUST_ESCALATE],
            affordance_bits: vec![],
            expected: Escalate,
            perturbations: vec![
                (Perturbation::DropRisk(ContextBit::MUST_ESCALATE), Ask),
            ],
        },
        CausalScenario {
            name: "ignore_via_calm_no_expectation_no_risk",
            profile: "core",
            field_ntriples: String::new(),
            posture_bits: vec![PostureBit::CALM],
            expectation_bits: vec![],
            risk_bits: vec![],
            affordance_bits: vec![],
            expected: Ignore,
            perturbations: vec![(Perturbation::DropPostureBit(PostureBit::CALM), Ask)],
        },
        CausalScenario {
            name: "ask_via_dd_type_evidence_gap_content_sensitive",
            profile: "enterprise",
            field_ntriples: "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n<http://example.org/doc1> <http://purl.org/dc/terms/title> \"Test Document\" .\n".to_string(),
            // DigitalDocument present with content makes Ask fire (evidence gap detected).
            // Removing the DD triple falls to Ignore (no gap). This proves the system
            // reads actual RDF structure, not a hardcoded presence counter.
            posture_bits: vec![PostureBit::CALM],
            expectation_bits: vec![],
            risk_bits: vec![],
            affordance_bits: vec![],
            expected: Ask,
            perturbations: vec![(
                Perturbation::DropTriple("<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> ."),
                Ignore
            )],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_scenarios_cover_every_response_class() {
        use std::collections::HashSet;
        let classes: HashSet<_> = canonical_scenarios().iter().map(|s| s.expected).collect();
        // 7-class lattice — every class must have a positive scenario.
        assert_eq!(classes.len(), 7, "missing coverage: {classes:?}");
    }

    #[test]
    fn each_scenario_has_at_least_one_perturbation() {
        for s in canonical_scenarios() {
            assert!(
                !s.perturbations.is_empty(),
                "scenario `{}` must declare ≥1 perturbation",
                s.name
            );
        }
    }
}
