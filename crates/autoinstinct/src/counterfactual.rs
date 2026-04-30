//! Phase 3 — Counterfactual generator.
//!
//! Given a [`crate::motifs::Motifs`] result, synthesize JTBD scenarios where
//! the perturbed context is a *different motif's context*. This is the
//! load-bearing-context-removal axis: if a candidate policy treats the two
//! motifs identically, it cannot be admitted.

use crate::jtbd::JtbdScenario;
use crate::motifs::Motifs;
use crate::AutonomicInstinct;

/// Generate scenarios by pairing each motif with a *different-response*
/// motif as its perturbation. Pairs are deterministic (motif index ascending).
#[must_use]
pub fn generate(motifs: &Motifs) -> Vec<JtbdScenario> {
    let mut out = Vec::new();
    for (i, m) in motifs.motifs.iter().enumerate() {
        let perturb = motifs
            .motifs
            .iter()
            .enumerate()
            .find(|(j, q)| *j != i && q.response != m.response)
            .map(|(_, q)| q.context_urn.clone())
            .unwrap_or_else(|| format!("urn:blake3:perturb-{}", i));
        out.push(JtbdScenario {
            name: format!("counterfactual-{}-{}", i, response_tag(m.response)),
            context_urn: m.context_urn.clone(),
            expected: m.response,
            perturbed_context_urn: perturb,
            forbidden: vec![],
        });
    }
    out
}

fn response_tag(r: AutonomicInstinct) -> &'static str {
    match r {
        AutonomicInstinct::Settle => "settle",
        AutonomicInstinct::Retrieve => "retrieve",
        AutonomicInstinct::Inspect => "inspect",
        AutonomicInstinct::Ask => "ask",
        AutonomicInstinct::Refuse => "refuse",
        AutonomicInstinct::Escalate => "escalate",
        AutonomicInstinct::Ignore => "ignore",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motifs::Motif;

    #[test]
    fn pairs_motifs_with_different_responses() {
        let motifs = Motifs {
            motifs: vec![
                Motif {
                    context_urn: "urn:blake3:a".into(),
                    response: AutonomicInstinct::Ask,
                    support: 5,
                },
                Motif {
                    context_urn: "urn:blake3:b".into(),
                    response: AutonomicInstinct::Inspect,
                    support: 3,
                },
            ],
        };
        let scenarios = generate(&motifs);
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].context_urn, "urn:blake3:a");
        assert_eq!(scenarios[0].perturbed_context_urn, "urn:blake3:b");
    }

    #[test]
    fn handles_single_motif_with_synthetic_perturbation() {
        let motifs = Motifs {
            motifs: vec![Motif {
                context_urn: "urn:blake3:solo".into(),
                response: AutonomicInstinct::Ask,
                support: 5,
            }],
        };
        let scenarios = generate(&motifs);
        assert_eq!(scenarios.len(), 1);
        assert!(scenarios[0]
            .perturbed_context_urn
            .starts_with("urn:blake3:perturb-"));
    }
}
