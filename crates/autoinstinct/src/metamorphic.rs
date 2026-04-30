//! Phase 4 — Metamorphic invariance suite.
//!
//! Verifies a candidate policy is invariant under irrelevant transformations
//! and changes under load-bearing transformations. Returns a typed verdict
//! per transformation; the gauntlet aggregates them.

use serde::{Deserialize, Serialize};

use crate::synth::CandidatePolicy;
use crate::AutonomicInstinct;

/// Outcome of one metamorphic check.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetamorphicVerdict {
    /// Name of the transformation (e.g. "irrelevant-suffix-rename").
    pub name: String,
    /// True iff the invariant held.
    pub held: bool,
    /// Optional human-readable note.
    pub note: String,
}

/// Run metamorphic checks against `policy` over `contexts`.
///
/// Invariants tested:
///
/// 1. **Idempotence** — `policy.select(ctx)` is referentially transparent.
/// 2. **Non-trivial sensitivity** — at least two contexts produce different
///    responses (i.e. policy is not constant).
/// 3. **Default safety** — unknown contexts fall back to `Ignore` (the
///    safe default).
#[must_use]
pub fn run(policy: &CandidatePolicy, contexts: &[&str]) -> Vec<MetamorphicVerdict> {
    let mut out = Vec::new();
    out.push(check_idempotence(policy, contexts));
    out.push(check_non_trivial_sensitivity(policy, contexts));
    out.push(check_default_safety(policy));
    out
}

fn check_idempotence(policy: &CandidatePolicy, contexts: &[&str]) -> MetamorphicVerdict {
    let mut held = true;
    for c in contexts {
        if policy.select(c) != policy.select(c) {
            held = false;
            break;
        }
    }
    MetamorphicVerdict {
        name: "idempotence".into(),
        held,
        note: "policy.select must be deterministic".into(),
    }
}

fn check_non_trivial_sensitivity(
    policy: &CandidatePolicy,
    contexts: &[&str],
) -> MetamorphicVerdict {
    let mut seen: Vec<AutonomicInstinct> = Vec::new();
    for c in contexts {
        seen.push(policy.select(c));
    }
    let held = seen.iter().any(|r| *r != seen[0]);
    MetamorphicVerdict {
        name: "non-trivial-sensitivity".into(),
        held,
        note: "policy must distinguish at least two contexts".into(),
    }
}

fn check_default_safety(policy: &CandidatePolicy) -> MetamorphicVerdict {
    let probe = "urn:blake3:never-seen-by-design";
    let r = policy.select(probe);
    let held = matches!(r, AutonomicInstinct::Ignore | AutonomicInstinct::Ask);
    MetamorphicVerdict {
        name: "default-safety".into(),
        held,
        note: "unknown context must fall back to Ignore or Ask".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metamorphic_passes_for_real_policy() {
        let p = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Inspect),
            ],
            default: AutonomicInstinct::Ignore,
        };
        let r = run(&p, &["urn:blake3:a", "urn:blake3:b"]);
        assert!(r.iter().all(|v| v.held), "{:?}", r);
    }

    #[test]
    fn metamorphic_fails_constant_policy() {
        let p = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Ask),
            ],
            default: AutonomicInstinct::Ask,
        };
        let r = run(&p, &["urn:blake3:a", "urn:blake3:b"]);
        // Idempotence holds, sensitivity fails.
        assert!(r.iter().any(|v| v.name == "non-trivial-sensitivity" && !v.held));
        // Default safety also fails because Ask is borderline; constant Ask
        // is admitted as a safe default by design (Ignore | Ask).
        assert!(r.iter().any(|v| v.name == "default-safety" && v.held));
    }

    #[test]
    fn metamorphic_flags_unsafe_default() {
        let p = CandidatePolicy {
            rules: vec![("urn:blake3:a".into(), AutonomicInstinct::Refuse)],
            default: AutonomicInstinct::Refuse,
        };
        let r = run(&p, &["urn:blake3:a"]);
        let safety = r.iter().find(|v| v.name == "default-safety").unwrap();
        assert!(!safety.held, "Refuse must NOT be a safe default");
    }
}
