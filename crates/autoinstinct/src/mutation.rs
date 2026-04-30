//! Phase 7 — Mutation-resistance harness.
//!
//! Mutates a candidate `CandidatePolicy` along load-bearing axes and
//! verifies the mutated policy fails the gauntlet. A mutation that
//! survives the gauntlet is a **semantic mutant** and signals the
//! gauntlet itself is too weak.

use serde::{Deserialize, Serialize};

use crate::gauntlet;
use crate::jtbd::JtbdScenario;
use crate::synth::CandidatePolicy;
use crate::AutonomicInstinct;

/// One mutation outcome.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MutationOutcome {
    /// Mutation name (e.g. "swap-first-two-rules").
    pub mutation: String,
    /// True iff the gauntlet caught the mutant.
    pub killed: bool,
}

/// Mutation-suite report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationReport {
    /// One outcome per mutation.
    pub outcomes: Vec<MutationOutcome>,
}

impl MutationReport {
    /// True iff every semantic mutant was killed by the gauntlet.
    #[must_use]
    pub fn all_killed(&self) -> bool {
        self.outcomes.iter().all(|o| o.killed)
    }
}

/// Run the mutation suite. The base policy must already pass the gauntlet —
/// otherwise mutation results are meaningless.
#[must_use]
pub fn run(base: &CandidatePolicy, scenarios: &[JtbdScenario]) -> MutationReport {
    let mut outcomes = Vec::new();

    // Mutation 1 — swap rule responses.
    if base.rules.len() >= 2 {
        let mut swapped = base.clone();
        let r0 = swapped.rules[0].1;
        let r1 = swapped.rules[1].1;
        swapped.rules[0].1 = r1;
        swapped.rules[1].1 = r0;
        let report = gauntlet::run(&swapped, scenarios);
        outcomes.push(MutationOutcome {
            mutation: "swap-first-two-rule-responses".into(),
            killed: !report.admitted(),
        });
    }

    // Mutation 2 — collapse all rules to a single response.
    let mut constant = base.clone();
    let target = if constant.default == AutonomicInstinct::Ask {
        AutonomicInstinct::Inspect
    } else {
        AutonomicInstinct::Ask
    };
    for r in &mut constant.rules {
        r.1 = target;
    }
    constant.default = target;
    let report = gauntlet::run(&constant, scenarios);
    outcomes.push(MutationOutcome {
        mutation: "collapse-to-constant-response".into(),
        killed: !report.admitted(),
    });

    // Mutation 3 — drop all rules.
    let stripped = CandidatePolicy {
        rules: vec![],
        default: base.default,
    };
    let report = gauntlet::run(&stripped, scenarios);
    outcomes.push(MutationOutcome {
        mutation: "drop-all-rules".into(),
        killed: !report.admitted(),
    });

    // Mutation 4 — replace default with Refuse (unsafe escalation).
    let mut bad_default = base.clone();
    bad_default.default = AutonomicInstinct::Refuse;
    let report = gauntlet::run(&bad_default, scenarios);
    outcomes.push(MutationOutcome {
        mutation: "default-to-refuse".into(),
        killed: !report.admitted(),
    });

    MutationReport { outcomes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_kills_constant_collapse() {
        let policy = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Inspect),
            ],
            default: AutonomicInstinct::Ignore,
        };
        let scenarios = vec![JtbdScenario {
            name: "a-vs-b".into(),
            context_urn: "urn:blake3:a".into(),
            expected: AutonomicInstinct::Ask,
            perturbed_context_urn: "urn:blake3:b".into(),
            forbidden: vec![AutonomicInstinct::Refuse],
        }];
        let report = run(&policy, &scenarios);
        let constant = report
            .outcomes
            .iter()
            .find(|o| o.mutation == "collapse-to-constant-response")
            .unwrap();
        assert!(constant.killed, "constant collapse must be killed");
        let drop = report
            .outcomes
            .iter()
            .find(|o| o.mutation == "drop-all-rules")
            .unwrap();
        assert!(drop.killed, "rule drop must be killed");
    }
}
