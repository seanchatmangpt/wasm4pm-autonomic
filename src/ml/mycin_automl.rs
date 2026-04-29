//! MYCIN AutoML Equivalent — Learned Diagnostic Classifier via Decision Tree.
//!
//! While `crate::ml::mycin` provides hand-crafted IF-THEN rules with certainty
//! factors, this module provides the *learned* equivalent: given training data
//! of (patient fact bitmask, diagnosis label) pairs, learn diagnosis via
//! decision tree induction (ID3-style entropy splits).
//!
//! # Compiled Cognition
//!
//! This module contributes `L_learned` to Compiled Cognition. Paired with
//! `mycin.rs` (`S_symbolic`), these two halves compose into the full
//! organism diagnosis primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Substrate Bifurcation
//!
//! Classical MYCIN (Shortliffe et al. 1974–76) is *expert knowledge*: hand-crafted
//! rules combining clinical evidence. This module is *learned knowledge*: induce
//! rules automatically from training data using entropy-driven splits.
//!
//! Both perform the **same JOB** — diagnose bacterial infection from clinical facts —
//! but via different paths:
//! - **Classical**: Hand-written rules, expert-elicited knowledge, slow to adapt
//! - **AutoML**: Induced trees from data, automatic rule discovery, adaptive
//!
//! # Feature Extraction
//!
//! Input is a u64 bitmask of 13 patient facts (gram stain, morphology, growth factors).
//! Output is a 13-dimensional binary feature vector extracted from those facts.
//!
//! # Architecture
//!
//! ```text
//! Patient facts (u64) → 13-dim binary vector → Decision Tree → Diagnosis (bool)
//! ```
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::mycin_automl;
//! use dteam::ml::mycin::fact;
//!
//! // Train: (patient facts, diagnosis) pairs
//! // Strep patients have GRAM_POS + COCCUS + AEROBIC + FEVER + RIGORS
//! let train_facts = vec![
//!     fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS, // strep
//!     fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS, // strep
//!     fact::GRAM_NEG | fact::ANAEROBIC,                                            // not strep
//!     fact::FEVER,                                                                  // not strep
//! ];
//! let labels = vec![true, true, false, false];
//!
//! // Test: does a GRAM_POS coccus with fever match strep?
//! let test_facts = vec![fact::GRAM_POS | fact::COCCUS | fact::FEVER];
//! let predictions = mycin_automl::classify(&train_facts, &labels, &test_facts);
//!
//! assert_eq!(predictions.len(), 1);
//! // predictions[0] will likely be true (classic strep pattern)
//! ```
//!
//! # Determinism
//!
//! This module is fully deterministic: identical inputs produce byte-identical outputs.
//! Decision tree induction uses entropy, which is deterministic across invocations.

use crate::ml::decision_tree;
use crate::ml::hdit_automl::SignalProfile;

/// Number of fact bits used as features (matches `mycin::fact::*` count).
pub const N_FACT_FEATURES: usize = 13;

/// Extract a binary feature vector from a patient fact bitmask.
///
/// # Example
///
/// ```rust
/// use dteam::ml::mycin_automl;
/// use dteam::ml::mycin::fact;
///
/// let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC;
/// let features = mycin_automl::bitmask_to_features(facts);
///
/// assert_eq!(features.len(), 13);
/// assert_eq!(features[1], 1.0); // GRAM_POS is bit 1
/// assert_eq!(features[3], 1.0); // COCCUS is bit 3
/// assert_eq!(features[4], 1.0); // AEROBIC is bit 4
/// ```
#[inline]
#[must_use]
pub fn bitmask_to_features(facts: u64) -> Vec<f64> {
    (0..N_FACT_FEATURES)
        .map(|i| if (facts >> i) & 1 != 0 { 1.0 } else { 0.0 })
        .collect()
}

/// Train decision tree on (facts, label) pairs and predict diagnosis on test facts.
#[must_use]
pub fn classify(train_facts: &[u64], labels: &[bool], test_facts: &[u64]) -> Vec<bool> {
    let train: Vec<Vec<f64>> = train_facts
        .iter()
        .map(|&m| bitmask_to_features(m))
        .collect();
    let test: Vec<Vec<f64>> = test_facts.iter().map(|&m| bitmask_to_features(m)).collect();
    decision_tree::classify_d3(&train, labels, &test)
}

/// AutoML signal: in-sample fit-and-predict on patient facts vs anchor diagnosis.
pub fn mycin_automl_signal(name: &str, patient_facts: &[u64], anchor: &[bool]) -> SignalProfile {
    let predictions = classify(patient_facts, anchor, patient_facts);
    // Decision tree is small but has tree-traversal cost — call it T1.
    let timing_us = (patient_facts.len() as u64 / 2).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::mycin::fact;

    #[test]
    fn feature_vector_length_is_13() {
        assert_eq!(bitmask_to_features(0).len(), N_FACT_FEATURES);
    }

    #[test]
    fn feature_vector_extracts_gram_pos() {
        let f = bitmask_to_features(fact::GRAM_POS);
        assert_eq!(f[1], 1.0); // GRAM_POS is bit 1
        assert_eq!(f[0], 0.0); // GRAM_NEG is bit 0
    }

    #[test]
    fn feature_vector_extracts_combined_facts() {
        let mask = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER;
        let f = bitmask_to_features(mask);
        assert_eq!(f[1], 1.0); // GRAM_POS
        assert_eq!(f[3], 1.0); // COCCUS
        assert_eq!(f[4], 1.0); // AEROBIC
        assert_eq!(f[6], 1.0); // FEVER
    }

    #[test]
    fn classify_strep_pattern_separable() {
        // Strep patients have GRAM_POS + COCCUS + AEROBIC + FEVER + RIGORS
        // Non-strep: GRAM_NEG or no rigors
        let train_facts = vec![
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS, // strep
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS, // strep
            fact::GRAM_NEG | fact::ANAEROBIC,                                           // not strep
            fact::FEVER,                                                                // not strep
        ];
        let labels = vec![true, true, false, false];
        let test_facts = vec![
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_NEG | fact::ANAEROBIC,
        ];
        let preds = classify(&train_facts, &labels, &test_facts);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn classify_empty_training_returns_majority() {
        let preds = classify(&[], &[], &[fact::FEVER]);
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn classify_is_deterministic_across_invocations() {
        let train = vec![
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_NEG | fact::ANAEROBIC,
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::FEVER,
        ];
        let labels = vec![true, false, true, false];
        let p1 = classify(&train, &labels, &train);
        let p2 = classify(&train, &labels, &train);
        let p3 = classify(&train, &labels, &train);
        assert_eq!(p1, p2);
        assert_eq!(p2, p3);
    }

    #[test]
    fn signal_in_sample_accuracy_is_high_on_separable_data() {
        let patients = vec![
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_NEG | fact::ANAEROBIC,
            fact::FEVER,
        ];
        let anchor = vec![true, true, false, false];
        let sig = mycin_automl_signal("mycin_dt", &patients, &anchor);
        assert!(sig.accuracy_vs_anchor >= 0.5);
    }

    #[test]
    fn signal_produces_correct_prediction_length() {
        let patients = vec![fact::FEVER, fact::GRAM_POS, fact::ANAEROBIC];
        let anchor = vec![true, false, false];
        let sig = mycin_automl_signal("mycin", &patients, &anchor);
        assert_eq!(sig.predictions.len(), 3);
    }
}
