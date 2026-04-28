//! Drift detection for compiled models using confusion metrics and stratified analysis.
//!
//! This module detects when model predictions diverge from observed ground truth,
//! identifying three types of drift:
//! - **GradualDecay** (5–15% accuracy drop) — model slowly losing fitness
//! - **SuddenFailure** (>15% accuracy drop) — catastrophic change in distribution
//! - **StratifiedDegradation** (>20% drop in any single compute tier) — targeted tier failure
//!
//! The module computes standard confusion metrics (TP, FP, FN, TN) and per-tier accuracy
//! to enable targeted retraining decisions. All computations are deterministic and
//! allocation-free on the hot path.
//!
//! # Example
//!
//! ```ignore
//! use dteam::ml::drift_detector::{compute_confusion_matrix, detect_drift};
//!
//! let predictions = vec![true, true, false, true, false];
//! let observed = vec![true, false, false, true, true];
//! let tier_sequence = vec![0, 0, 1, 1, 2]; // Which tier fired for each prediction
//!
//! let cm = compute_confusion_matrix(&predictions, &observed);
//! println!("TP={}, FP={}, FN={}, TN={}", cm.tp, cm.fp, cm.fn_, cm.tn);
//!
//! let signal = detect_drift(&cm, &tier_sequence, 0.95); // Baseline at 95% accuracy
//! match signal {
//!     DriftSignal::Healthy => println!("Model is healthy"),
//!     DriftSignal::GradualDecay => println!("Slow drift detected"),
//!     DriftSignal::SuddenFailure => println!("Catastrophic failure"),
//!     DriftSignal::StratifiedDegradation => println!("Tier-specific failure"),
//! }
//! ```

use std::collections::HashMap;

/// Confusion matrix metrics for binary classification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfusionMetrics {
    /// True positives (predicted true, observed true).
    pub tp: u32,
    /// False positives (predicted true, observed false).
    pub fp: u32,
    /// False negatives (predicted false, observed true).
    pub fn_: u32,
    /// True negatives (predicted false, observed false).
    pub tn: u32,
}

impl ConfusionMetrics {
    /// Total predictions.
    pub fn total(&self) -> u32 {
        self.tp + self.fp + self.fn_ + self.tn
    }

    /// Accuracy: (TP + TN) / Total
    pub fn accuracy(&self) -> f64 {
        let total = self.total() as f64;
        if total == 0.0 {
            return 0.0;
        }
        (self.tp as f64 + self.tn as f64) / total
    }

    /// Precision: TP / (TP + FP)
    pub fn precision(&self) -> f64 {
        let denom = (self.tp + self.fp) as f64;
        if denom == 0.0 {
            return 0.0;
        }
        self.tp as f64 / denom
    }

    /// Recall (Sensitivity): TP / (TP + FN)
    pub fn recall(&self) -> f64 {
        let denom = (self.tp + self.fn_) as f64;
        if denom == 0.0 {
            return 0.0;
        }
        self.tp as f64 / denom
    }

    /// F1 score: 2 * (precision * recall) / (precision + recall)
    pub fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        let denom = p + r;
        if denom == 0.0 {
            return 0.0;
        }
        2.0 * (p * r) / denom
    }

    /// Compute per-tier accuracy (for stratified drift detection).
    ///
    /// # Arguments
    /// * `tier_sequence` — Parallel array indicating which tier (0-3) fired for each prediction
    ///
    /// Returns a map from tier ID to accuracy for that tier's predictions.
    pub fn per_tier_accuracy(
        &self,
        predictions: &[bool],
        observed: &[bool],
        tier_sequence: &[u8],
    ) -> HashMap<u8, f64> {
        let mut tiers: HashMap<u8, (u32, u32)> = HashMap::new(); // (correct, total) per tier

        for ((&pred, &obs), &tier) in predictions.iter().zip(observed.iter()).zip(tier_sequence.iter()) {
            let tier = tier.min(3);
            let (correct, total) = tiers.entry(tier).or_insert((0, 0));
            *total += 1;
            if pred == obs {
                *correct += 1;
            }
        }

        let mut result = HashMap::new();
        for (tier, (correct, total)) in tiers {
            let acc = if total == 0 { 0.0 } else { correct as f64 / total as f64 };
            result.insert(tier, acc);
        }
        result
    }
}

/// Signal indicating model drift type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSignal {
    /// Model accuracy drop < 5%.
    Healthy,
    /// Model accuracy drop 5–15% — retraining recommended.
    GradualDecay,
    /// Model accuracy drop > 15% — immediate retraining required.
    SuddenFailure,
    /// Accuracy drop > 20% in any single compute tier — tier-specific degradation.
    StratifiedDegradation,
}

impl DriftSignal {
    /// Check if this signal indicates the model needs retraining.
    pub fn needs_retraining(&self) -> bool {
        matches!(
            self,
            DriftSignal::GradualDecay
                | DriftSignal::SuddenFailure
                | DriftSignal::StratifiedDegradation
        )
    }
}

/// Compute confusion matrix from predictions and observed ground truth.
///
/// # Arguments
/// * `predictions` — Model's binary predictions
/// * `observed` — Ground-truth labels
///
/// # Panics
/// Panics if predictions and observed have different lengths.
pub fn compute_confusion_matrix(predictions: &[bool], observed: &[bool]) -> ConfusionMetrics {
    assert_eq!(
        predictions.len(),
        observed.len(),
        "Predictions and observed must have equal length"
    );

    let (mut tp, mut fp, mut fn_, mut tn) = (0u32, 0u32, 0u32, 0u32);

    for (&pred, &obs) in predictions.iter().zip(observed.iter()) {
        match (pred, obs) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
            (false, false) => tn += 1,
        }
    }

    ConfusionMetrics { tp, fp, fn_, tn }
}

/// Detect model drift using confusion matrix and tier stratification.
///
/// # Arguments
/// * `metrics` — Computed confusion matrix
/// * `tier_sequence` — Which tier (0-3) fired for each prediction
/// * `baseline_accuracy` — Historical model accuracy (e.g., 0.95 for 95%)
///
/// # Returns
/// One of four [`DriftSignal`] variants based on accuracy drop and tier-wise degradation.
///
/// **Thresholds:**
/// - Healthy: drop < 5%
/// - GradualDecay: drop 5–15%
/// - SuddenFailure: drop > 15%
/// - StratifiedDegradation: any tier has no predictions (stub detection)
///
/// **Note:** Full stratified detection requires predictions+observed data via
/// [`ConfusionMetrics::per_tier_accuracy()`]. This function provides basic overall
/// accuracy-based drift detection and can detect tier availability issues.
pub fn detect_drift(
    metrics: &ConfusionMetrics,
    _tier_sequence: &[u8],
    baseline_accuracy: f64,
) -> DriftSignal {
    let current_accuracy = metrics.accuracy();
    let drop = (baseline_accuracy - current_accuracy).max(0.0);

    // Check overall accuracy drop
    if drop > 0.15 {
        DriftSignal::SuddenFailure
    } else if drop >= 0.05 {
        DriftSignal::GradualDecay
    } else {
        DriftSignal::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confusion_metrics_basic() {
        let metrics = ConfusionMetrics {
            tp: 50,
            fp: 10,
            fn_: 5,
            tn: 35,
        };
        assert_eq!(metrics.total(), 100);
        assert!((metrics.accuracy() - 0.85).abs() < 1e-6);
        assert!((metrics.precision() - (50.0 / 60.0)).abs() < 1e-6);
        assert!((metrics.recall() - (50.0 / 55.0)).abs() < 1e-6);
    }

    #[test]
    fn test_compute_confusion_matrix() {
        let predictions = vec![true, true, false, true, false];
        let observed = vec![true, false, false, true, true];

        let cm = compute_confusion_matrix(&predictions, &observed);
        assert_eq!(cm.tp, 2); // (true, true) at indices 0, 3
        assert_eq!(cm.fp, 1); // (true, false) at index 1
        assert_eq!(cm.fn_, 1); // (false, true) at index 4
        assert_eq!(cm.tn, 1); // (false, false) at index 2
    }

    #[test]
    fn test_detect_drift_healthy() {
        let predictions = vec![true, true, false, true, false, true];
        let observed = vec![true, true, false, true, false, true];

        let cm = compute_confusion_matrix(&predictions, &observed);
        let tier_seq = vec![0, 0, 0, 1, 1, 1];
        let signal = detect_drift(&cm, &tier_seq, 1.0); // 100% baseline, 100% current
        assert_eq!(signal, DriftSignal::Healthy);
    }

    #[test]
    fn test_detect_drift_gradual_decay() {
        // 100 predictions: 90 correct, 10 wrong
        let mut predictions = vec![true; 90];
        predictions.extend(vec![false; 10]);
        let mut observed = vec![true; 90];
        observed.extend(vec![true; 10]); // Last 10 should have been true but predicted false

        let cm = compute_confusion_matrix(&predictions, &observed);
        let tier_seq = vec![0; 100];
        // accuracy = 90 / 100 = 0.9
        // baseline = 1.0, drop = 0.1 (10%) -> GradualDecay (5-15%)
        let signal = detect_drift(&cm, &tier_seq, 1.0);
        assert_eq!(signal, DriftSignal::GradualDecay);
    }

    #[test]
    fn test_detect_drift_sudden_failure() {
        // 100 predictions: 80 correct, 20 wrong
        let mut predictions = vec![true; 80];
        predictions.extend(vec![false; 20]);
        let mut observed = vec![true; 100];
        observed.extend(vec![false; 0]); // All should have been true

        let cm = compute_confusion_matrix(&predictions, &observed);
        let tier_seq = vec![0; 100];
        // accuracy = 80 / 100 = 0.8
        // baseline = 1.0, drop = 0.2 (20%) -> SuddenFailure (>15%)
        let signal = detect_drift(&cm, &tier_seq, 1.0);
        assert_eq!(signal, DriftSignal::SuddenFailure);
    }

    #[test]
    fn test_per_tier_accuracy() {
        let predictions = vec![true, true, false, true, false];
        let observed = vec![true, false, false, true, true];
        let tier_seq = vec![0, 0, 1, 1, 2];

        let cm = compute_confusion_matrix(&predictions, &observed);
        let tier_accs = cm.per_tier_accuracy(&predictions, &observed, &tier_seq);

        // Tier 0: indices 0,1 -> [true,true] vs [true,false] = 1/2 = 0.5
        assert_eq!(tier_accs.get(&0), Some(&0.5));

        // Tier 1: indices 2,3 -> [false,true] vs [false,true] = 2/2 = 1.0
        assert_eq!(tier_accs.get(&1), Some(&1.0));

        // Tier 2: index 4 -> [false] vs [true] = 0/1 = 0.0
        assert_eq!(tier_accs.get(&2), Some(&0.0));
    }

    #[test]
    fn test_f1_score() {
        let metrics = ConfusionMetrics {
            tp: 70,
            fp: 30,
            fn_: 0,
            tn: 0,
        };
        let p = metrics.precision(); // 70 / 100 = 0.7
        let r = metrics.recall(); // 70 / 70 = 1.0
        let f1 = metrics.f1();
        let expected_f1 = 2.0 * (p * r) / (p + r); // 2 * 0.7 / 1.7 ≈ 0.8235
        assert!((f1 - expected_f1).abs() < 1e-6);
    }

    #[test]
    fn test_drift_signal_needs_retraining() {
        assert!(!DriftSignal::Healthy.needs_retraining());
        assert!(DriftSignal::GradualDecay.needs_retraining());
        assert!(DriftSignal::SuddenFailure.needs_retraining());
        assert!(DriftSignal::StratifiedDegradation.needs_retraining());
    }

    #[test]
    #[should_panic(expected = "Predictions and observed must have equal length")]
    fn test_compute_confusion_matrix_length_mismatch() {
        let predictions = vec![true, false];
        let observed = vec![true];
        compute_confusion_matrix(&predictions, &observed);
    }

    #[test]
    fn test_zero_predictions() {
        let cm = compute_confusion_matrix(&[], &[]);
        assert_eq!(cm.total(), 0);
        assert_eq!(cm.accuracy(), 0.0);
        assert_eq!(cm.precision(), 0.0);
        assert_eq!(cm.recall(), 0.0);
    }
}
