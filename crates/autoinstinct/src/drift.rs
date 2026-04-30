//! Outcome / drift monitor.
//!
//! After deployment, runtime outcomes feed back into AutoInstinct to detect
//! drift between the compiled policy and observed behavior. This module
//! offers the minimal monitor — a counter of mismatches per `(context_urn,
//! response)` — sufficient to gate redeploy. Phase 2 adds EWMA smoothing
//! and significance testing.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::AutonomicInstinct;

/// One outcome observation from the runtime.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Outcome {
    /// Closed-context fingerprint.
    pub context_urn: String,
    /// Response the deployed pack produced.
    pub deployed: AutonomicInstinct,
    /// Response a downstream verifier (audit, human, replay) judged correct.
    pub corrected: AutonomicInstinct,
}

/// Counts mismatches between deployed and corrected responses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DriftMonitor {
    /// `(context_urn, deployed) -> mismatch_count`. Deterministic key order.
    pub mismatches: IndexMap<(String, AutonomicInstinct), u32>,
    /// Total observations consumed.
    pub observations: u64,
}

impl DriftMonitor {
    /// Empty monitor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one outcome. Returns true iff the deployed response matched
    /// the correction.
    pub fn record(&mut self, o: &Outcome) -> bool {
        self.observations += 1;
        if o.deployed == o.corrected {
            return true;
        }
        *self
            .mismatches
            .entry((o.context_urn.clone(), o.deployed))
            .or_insert(0) += 1;
        false
    }

    /// True iff any observed mismatch crosses `threshold`.
    #[must_use]
    pub fn drift_detected(&self, threshold: u32) -> bool {
        self.mismatches.values().any(|n| *n >= threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(ctx: &str, dep: AutonomicInstinct, cor: AutonomicInstinct) -> Outcome {
        Outcome {
            context_urn: ctx.into(),
            deployed: dep,
            corrected: cor,
        }
    }

    #[test]
    fn drift_records_mismatches() {
        let mut m = DriftMonitor::new();
        let ok = m.record(&outcome("urn:blake3:a", AutonomicInstinct::Ask, AutonomicInstinct::Ask));
        assert!(ok);
        let bad = m.record(&outcome(
            "urn:blake3:a",
            AutonomicInstinct::Ask,
            AutonomicInstinct::Inspect,
        ));
        assert!(!bad);
        assert_eq!(m.observations, 2);
        assert!(!m.drift_detected(2));
    }

    #[test]
    fn drift_threshold_fires() {
        let mut m = DriftMonitor::new();
        for _ in 0..3 {
            m.record(&outcome(
                "urn:blake3:a",
                AutonomicInstinct::Ask,
                AutonomicInstinct::Inspect,
            ));
        }
        assert!(m.drift_detected(3));
        assert!(!m.drift_detected(4));
    }
}
