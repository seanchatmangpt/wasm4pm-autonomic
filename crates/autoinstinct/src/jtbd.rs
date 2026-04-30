//! Generated JTBD scenarios.
//!
//! A `JtbdScenario` carries the positive/negative/perturbation triad in a
//! data structure. The gauntlet evaluates a candidate policy against the
//! scenario without ever calling user code stubs — it asserts the policy's
//! output against the scenario's expected response, then asserts a
//! perturbation produces a different response.

use serde::{Deserialize, Serialize};

use crate::AutonomicInstinct;

/// A single JTBD scenario.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JtbdScenario {
    /// Human-readable scenario name.
    pub name: String,
    /// Closed-context fingerprint for the positive case.
    pub context_urn: String,
    /// Expected response when context is present.
    pub expected: AutonomicInstinct,
    /// Closed-context fingerprint after one load-bearing input is removed.
    pub perturbed_context_urn: String,
    /// Forbidden response classes — these must NEVER be produced for
    /// either the positive or perturbed case (e.g. dev pack must never
    /// emit `Refuse` or `Escalate`).
    pub forbidden: Vec<AutonomicInstinct>,
}

/// Outcome of evaluating one scenario.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JtbdResult {
    /// Scenario name (echoed for reporting).
    pub name: String,
    /// True iff positive case produced expected response.
    pub positive_ok: bool,
    /// True iff perturbed case produced a *different* response than positive.
    pub perturbation_ok: bool,
    /// True iff neither case produced a forbidden response.
    pub boundary_ok: bool,
}

impl JtbdResult {
    /// True iff every triad assertion held.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.positive_ok && self.perturbation_ok && self.boundary_ok
    }
}

/// Evaluate a scenario against a `select` callback (e.g. a candidate policy
/// or a `ccog` field-pack `select_instinct`).
pub fn evaluate<F>(scenario: &JtbdScenario, select: F) -> JtbdResult
where
    F: Fn(&str) -> AutonomicInstinct,
{
    let positive = select(&scenario.context_urn);
    let perturbed = select(&scenario.perturbed_context_urn);
    let positive_ok = positive == scenario.expected;
    let perturbation_ok = positive != perturbed;
    let boundary_ok = !scenario.forbidden.contains(&positive)
        && !scenario.forbidden.contains(&perturbed);
    JtbdResult {
        name: scenario.name.clone(),
        positive_ok,
        perturbation_ok,
        boundary_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jtbd_evaluation_catches_stub_policies() {
        // A stub policy that returns Ask for everything.
        let stub = |_ctx: &str| AutonomicInstinct::Ask;
        let scenario = JtbdScenario {
            name: "fatigue softens".into(),
            context_urn: "urn:blake3:fatigue".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:no-fatigue".into(),
            forbidden: vec![AutonomicInstinct::Refuse, AutonomicInstinct::Escalate],
        };
        let r = evaluate(&scenario, stub);
        // Stub passes positive + boundary, but fails perturbation: same answer twice.
        assert!(r.positive_ok);
        assert!(r.boundary_ok);
        assert!(!r.perturbation_ok, "stub must fail perturbation triad");
        assert!(!r.passed());
    }

    #[test]
    fn jtbd_evaluation_admits_genuine_policy() {
        let policy = |ctx: &str| -> AutonomicInstinct {
            if ctx == "urn:blake3:fatigue" {
                AutonomicInstinct::Ask
            } else {
                AutonomicInstinct::Inspect
            }
        };
        let scenario = JtbdScenario {
            name: "fatigue softens".into(),
            context_urn: "urn:blake3:fatigue".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:no-fatigue".into(),
            forbidden: vec![AutonomicInstinct::Refuse, AutonomicInstinct::Escalate],
        };
        let r = evaluate(&scenario, policy);
        assert!(r.passed());
    }

    #[test]
    fn jtbd_evaluation_catches_forbidden_response() {
        let policy = |_ctx: &str| AutonomicInstinct::Refuse;
        let scenario = JtbdScenario {
            name: "dev never refuses".into(),
            context_urn: "urn:blake3:dev".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:dev-no-issue".into(),
            forbidden: vec![AutonomicInstinct::Refuse, AutonomicInstinct::Escalate],
        };
        let r = evaluate(&scenario, policy);
        assert!(!r.boundary_ok, "Refuse must trip the forbidden-class boundary");
        assert!(!r.passed());
    }
}
