//! Candidate μ policy synthesis.
//!
//! A candidate policy maps closed-context fingerprints to canonical
//! `AutonomicInstinct` response classes. Synthesis takes [`crate::motifs::Motifs`]
//! and produces a deterministic [`CandidatePolicy`] suitable for input to
//! the gauntlet.
//!
//! AutoInstinct never forks the response lattice — every emitted decision
//! must be one of the seven canonical classes.

use serde::{Deserialize, Serialize};

use crate::motifs::Motifs;
use crate::AutonomicInstinct;

/// Candidate μ policy.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CandidatePolicy {
    /// `(context_urn, response)` rules in deterministic order. First match wins.
    pub rules: Vec<(String, AutonomicInstinct)>,
    /// Default fallback class used when no rule matches.
    pub default: AutonomicInstinct,
}

impl CandidatePolicy {
    /// Look up the canonical response for a context fingerprint.
    /// Returns `default` when no rule matches.
    #[must_use]
    pub fn select(&self, context_urn: &str) -> AutonomicInstinct {
        for (ctx, r) in &self.rules {
            if ctx == context_urn {
                return *r;
            }
        }
        self.default
    }
}

/// Synthesize a candidate policy from motifs. Deterministic and total —
/// every motif becomes a rule; the most-supported `Ignore` class is used
/// as the safe default when no motif matches.
#[must_use]
pub fn synthesize(motifs: &Motifs) -> CandidatePolicy {
    let mut rules: Vec<(String, AutonomicInstinct)> = motifs
        .motifs
        .iter()
        .map(|m| (m.context_urn.clone(), m.response))
        .collect();
    rules.sort_by(|a, b| a.0.cmp(&b.0));
    CandidatePolicy {
        rules,
        default: AutonomicInstinct::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motifs::Motif;

    #[test]
    fn synth_emits_canonical_lattice_only() {
        let m = Motifs {
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
        let p = synthesize(&m);
        assert_eq!(p.select("urn:blake3:a"), AutonomicInstinct::Ask);
        assert_eq!(p.select("urn:blake3:b"), AutonomicInstinct::Inspect);
        assert_eq!(p.select("urn:blake3:unknown"), AutonomicInstinct::Ignore);
        // Constitutional check: every rule resolves to a canonical variant.
        for (_, r) in &p.rules {
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
    }
}
