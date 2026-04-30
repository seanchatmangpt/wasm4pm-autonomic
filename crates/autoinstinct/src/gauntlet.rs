//! Gauntlet — admit/deny gate over candidate μ policies.
//!
//! The gauntlet runs every required test surface and produces a
//! [`GauntletReport`]. A candidate is admitted only when every surface
//! passes; any failure produces a counterexample.
//!
//! The current implementation enforces the JTBD triad (positive +
//! perturbation + forbidden-class boundary) over generated scenarios. As
//! Phase 2 lands, additional surfaces (mutation, fuzzing, replay, receipt
//! sensitivity, POWL64 tamper, benchmark tier) plug in here.

use serde::{Deserialize, Serialize};

use crate::jtbd::{evaluate, JtbdResult, JtbdScenario};
use crate::synth::CandidatePolicy;

/// One counterexample failing the gauntlet.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counterexample {
    /// Scenario name.
    pub scenario: String,
    /// Which assertion failed: "positive" | "perturbation" | "boundary".
    pub surface: String,
}

/// Outcome of running the gauntlet over a candidate.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GauntletReport {
    /// Per-scenario results.
    pub results: Vec<JtbdResult>,
    /// All counterexamples (empty iff admitted).
    pub counterexamples: Vec<Counterexample>,
}

impl GauntletReport {
    /// True iff no counterexamples were produced.
    #[must_use]
    pub fn admitted(&self) -> bool {
        self.counterexamples.is_empty()
    }
}

/// Run the JTBD gauntlet over a candidate policy. Returns a report; admit
/// the candidate only if [`GauntletReport::admitted`].
#[must_use]
pub fn run(policy: &CandidatePolicy, scenarios: &[JtbdScenario]) -> GauntletReport {
    let mut report = GauntletReport::default();
    for s in scenarios {
        let r = evaluate(s, |ctx| policy.select(ctx));
        if !r.positive_ok {
            report.counterexamples.push(Counterexample {
                scenario: r.name.clone(),
                surface: "positive".into(),
            });
        }
        if !r.perturbation_ok {
            report.counterexamples.push(Counterexample {
                scenario: r.name.clone(),
                surface: "perturbation".into(),
            });
        }
        if !r.boundary_ok {
            report.counterexamples.push(Counterexample {
                scenario: r.name.clone(),
                surface: "boundary".into(),
            });
        }
        report.results.push(r);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AutonomicInstinct;

    #[test]
    fn gauntlet_admits_correct_policy() {
        let policy = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Inspect),
            ],
            default: AutonomicInstinct::Ignore,
        };
        let scenarios = vec![JtbdScenario {
            name: "a vs b".into(),
            context_urn: "urn:blake3:a".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:b".into(),
            forbidden: vec![AutonomicInstinct::Refuse],
        }];
        let report = run(&policy, &scenarios);
        assert!(report.admitted(), "{:?}", report.counterexamples);
    }

    #[test]
    fn gauntlet_rejects_constant_policy() {
        // A policy that returns the same response for everything fails
        // perturbation by construction.
        let policy = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Ask),
            ],
            default: AutonomicInstinct::Ask,
        };
        let scenarios = vec![JtbdScenario {
            name: "constant".into(),
            context_urn: "urn:blake3:a".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:b".into(),
            forbidden: vec![],
        }];
        let report = run(&policy, &scenarios);
        assert!(!report.admitted());
        assert!(report
            .counterexamples
            .iter()
            .any(|c| c.surface == "perturbation"));
    }
}
