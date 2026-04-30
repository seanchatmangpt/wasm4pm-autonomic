//! Phase 8 — Streaming drift detector.
//!
//! Detects drift online without keeping the full corpus. Uses an
//! exponentially weighted moving average of mismatch rate and fires when
//! the EWMA crosses a threshold.

use serde::{Deserialize, Serialize};

use crate::drift::Outcome;

/// EWMA-based streaming drift detector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamingDrift {
    /// Smoothing factor in (0, 1]. Higher = more reactive.
    pub alpha: f64,
    /// EWMA of the mismatch indicator (1 = mismatch, 0 = match).
    pub ewma_mismatch: f64,
    /// Threshold above which `is_drifting` returns true.
    pub threshold: f64,
    /// Number of observations consumed.
    pub observations: u64,
}

impl StreamingDrift {
    /// Construct a detector with `alpha` smoothing and `threshold` EWMA.
    #[must_use]
    pub fn new(alpha: f64, threshold: f64) -> Self {
        debug_assert!(alpha > 0.0 && alpha <= 1.0);
        Self {
            alpha,
            ewma_mismatch: 0.0,
            threshold,
            observations: 0,
        }
    }

    /// Consume one outcome. Returns the current EWMA after the update.
    pub fn observe(&mut self, o: &Outcome) -> f64 {
        let indicator = if o.deployed == o.corrected { 0.0 } else { 1.0 };
        self.ewma_mismatch =
            self.alpha * indicator + (1.0 - self.alpha) * self.ewma_mismatch;
        self.observations += 1;
        self.ewma_mismatch
    }

    /// True iff the EWMA mismatch rate has crossed the threshold.
    #[must_use]
    pub fn is_drifting(&self) -> bool {
        self.ewma_mismatch > self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AutonomicInstinct;

    fn ok() -> Outcome {
        Outcome {
            context_urn: "urn:blake3:a".into(),
            deployed: AutonomicInstinct::Ask,
            corrected: AutonomicInstinct::Ask,
        }
    }
    fn bad() -> Outcome {
        Outcome {
            context_urn: "urn:blake3:a".into(),
            deployed: AutonomicInstinct::Ask,
            corrected: AutonomicInstinct::Inspect,
        }
    }

    #[test]
    fn streaming_does_not_fire_on_steady_state() {
        let mut s = StreamingDrift::new(0.3, 0.5);
        for _ in 0..50 {
            s.observe(&ok());
        }
        assert!(!s.is_drifting());
    }

    #[test]
    fn streaming_fires_on_persistent_mismatch() {
        let mut s = StreamingDrift::new(0.3, 0.5);
        for _ in 0..50 {
            s.observe(&bad());
        }
        assert!(s.is_drifting());
    }

    #[test]
    fn streaming_recovers_after_correction() {
        let mut s = StreamingDrift::new(0.3, 0.5);
        for _ in 0..50 {
            s.observe(&bad());
        }
        assert!(s.is_drifting());
        for _ in 0..50 {
            s.observe(&ok());
        }
        assert!(!s.is_drifting());
    }
}
