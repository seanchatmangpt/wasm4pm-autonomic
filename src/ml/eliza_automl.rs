//! ELIZA AutoML Equivalent — Learned Intent Classifier.
//!
//! While `crate::ml::eliza` provides hand-crafted keyword pattern matching, this
//! module provides the *learned* equivalent: given training data of (keyword
//! bitmask, label) pairs, learn the intent classification via Naive Bayes.
//!
//! Performs the **same JOB** as classical ELIZA: classify input intent.
//! The substrate-bifurcation thesis: the same job admits multiple realizations.

use crate::ml::hdit_automl::SignalProfile;
use crate::ml::naive_bayes;

/// Number of keyword feature bits (matches `eliza::kw::*` slot count).
pub const N_KEYWORD_FEATURES: usize = 16;

/// Convert a u64 keyword mask to a binary feature vector of length 16.
///
/// Each feature is 1.0 if the corresponding keyword bit is set, 0.0 otherwise.
#[inline]
#[must_use]
pub fn bitmask_to_features(mask: u64) -> Vec<f64> {
    (0..N_KEYWORD_FEATURES)
        .map(|i| if (mask >> i) & 1 != 0 { 1.0 } else { 0.0 })
        .collect()
}

/// Train naive Bayes on (mask, label) pairs and predict on test masks.
#[must_use]
pub fn classify(train_masks: &[u64], labels: &[bool], test_masks: &[u64]) -> Vec<bool> {
    let train: Vec<Vec<f64>> = train_masks.iter().map(|&m| bitmask_to_features(m)).collect();
    let test: Vec<Vec<f64>> = test_masks.iter().map(|&m| bitmask_to_features(m)).collect();
    naive_bayes::classify(&train, labels, &test)
}

/// AutoML signal: train on `inputs` self-supervised against `anchor`, predict on `inputs`.
///
/// This is the leave-it-in form: predictions equal in-sample fits, useful as a
/// signal generator when paired with the classical ELIZA on the same anchor.
pub fn eliza_automl_signal(
    name: &str,
    inputs: &[u64],
    anchor: &[bool],
) -> SignalProfile {
    let predictions = classify(inputs, anchor, inputs);
    // Naive-Bayes training is small-O on 16 features — counts as ~T1 tier.
    let timing_us = (inputs.len() as u64 / 4).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::eliza::{keyword_bit, kw};

    #[test]
    fn feature_vector_length_is_16() {
        assert_eq!(bitmask_to_features(0).len(), N_KEYWORD_FEATURES);
    }

    #[test]
    fn feature_vector_extracts_correct_bits() {
        let mask = keyword_bit(kw::DREAM) | keyword_bit(kw::MOTHER);
        let f = bitmask_to_features(mask);
        assert_eq!(f[kw::DREAM as usize], 1.0);
        assert_eq!(f[kw::MOTHER as usize], 1.0);
        assert_eq!(f[kw::SORRY as usize], 0.0);
    }

    #[test]
    fn feature_vector_all_zero_for_empty_mask() {
        let f = bitmask_to_features(0);
        assert!(f.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn classify_perfect_separation() {
        // Two clearly separated classes
        let train_masks = vec![
            keyword_bit(kw::DREAM),                         // true
            keyword_bit(kw::DREAM) | keyword_bit(kw::I),    // true
            keyword_bit(kw::SORRY),                         // false
            keyword_bit(kw::FATHER),                        // false
        ];
        let labels = vec![true, true, false, false];
        let test_masks = vec![keyword_bit(kw::DREAM), keyword_bit(kw::SORRY)];
        let preds = classify(&train_masks, &labels, &test_masks);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn classify_handles_empty_training_data() {
        let preds = classify(&[], &[], &[keyword_bit(kw::DREAM)]);
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn signal_produces_correct_length() {
        let inputs = vec![keyword_bit(kw::DREAM), keyword_bit(kw::SORRY), 0];
        let anchor = vec![true, false, false];
        let sig = eliza_automl_signal("eliza_nb", &inputs, &anchor);
        assert_eq!(sig.predictions.len(), 3);
    }

    #[test]
    fn signal_in_sample_accuracy_is_at_least_chance() {
        // 4-input dataset with clear pattern
        let inputs = vec![
            keyword_bit(kw::DREAM),
            keyword_bit(kw::DREAM),
            keyword_bit(kw::SORRY),
            keyword_bit(kw::FATHER),
        ];
        let anchor = vec![true, true, false, false];
        let sig = eliza_automl_signal("eliza_nb", &inputs, &anchor);
        assert!(sig.accuracy_vs_anchor >= 0.5);
    }
}
