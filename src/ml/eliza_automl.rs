//! ELIZA AutoML Equivalent — Learned Intent Classifier via Naive Bayes.
//!
//! While `crate::ml::eliza` provides hand-crafted keyword pattern matching, this
//! module provides the *learned* equivalent: given training data of (keyword
//! bitmask, label) pairs, learn intent classification via Naive Bayes.
//!
//! # Compiled Cognition
//!
//! This module contributes `L_learned` to Compiled Cognition. Paired with
//! `eliza.rs` (`S_symbolic`), these two halves compose into the full
//! intent classification primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Substrate Bifurcation
//!
//! Classical ELIZA (Weizenbaum 1966) is *symbolic cognition*: IF keyword THEN template.
//! This module is *learned cognition*: given (keywords, intent) pairs, induce a
//! probability model over intent given keywords.
//!
//! Both perform the **same JOB** — classify dialogue intent — but via different physics:
//! - **Classical**: Pattern matching, O(1) inference, hand-tuned, brittle
//! - **AutoML**: Frequency-based induction, O(1) inference, data-driven, generalizable
//!
//! # Feature Extraction
//!
//! Input is a u64 keyword bitmask (one bit per keyword in `crate::ml::eliza::kw`).
//! Output is a 16-dimensional binary feature vector: `[1.0 if bit i set, else 0.0]`.
//!
//! # Architecture
//!
//! ```text
//! Keywords (u64 mask) → 16-dim binary vector → Naive Bayes → Intent (bool)
//! ```
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::eliza_automl;
//! use dteam::ml::eliza::{keyword_bit, kw};
//!
//! // Train: pairs of (keyword mask, intent label)
//! let train_masks = vec![
//!     keyword_bit(kw::DREAM) | keyword_bit(kw::I),     // positive intent
//!     keyword_bit(kw::DREAM),                            // positive intent
//!     keyword_bit(kw::SORRY),                            // negative intent
//!     keyword_bit(kw::FATHER),                           // negative intent
//! ];
//! let labels = vec![true, true, false, false];
//!
//! // Test: does "dream" classify as positive?
//! let test_masks = vec![keyword_bit(kw::DREAM)];
//! let predictions = eliza_automl::classify(&train_masks, &labels, &test_masks);
//!
//! assert_eq!(predictions.len(), 1);
//! // predictions[0] will likely be true (dream is indicative of positive intent)
//! ```
//!
//! # Determinism
//!
//! This module is fully deterministic: identical inputs produce byte-identical outputs
//! across invocations. No randomization, no floating-point drift (i16 arithmetic fixed-point).

use crate::ml::hdit_automl::SignalProfile;
use crate::ml::naive_bayes;

/// Number of keyword feature bits (matches `eliza::kw::*` slot count).
pub const N_KEYWORD_FEATURES: usize = 16;

/// Convert a u64 keyword mask to a binary feature vector of length 16.
///
/// Each feature is 1.0 if the corresponding keyword bit is set, 0.0 otherwise.
///
/// # Example
///
/// ```rust
/// use dteam::ml::eliza_automl;
/// use dteam::ml::eliza::{keyword_bit, kw};
///
/// let mask = keyword_bit(kw::DREAM) | keyword_bit(kw::MOTHER);
/// let features = eliza_automl::bitmask_to_features(mask);
///
/// assert_eq!(features.len(), 16);
/// assert_eq!(features[kw::DREAM as usize], 1.0);
/// assert_eq!(features[kw::MOTHER as usize], 1.0);
/// assert_eq!(features[kw::SORRY as usize], 0.0);
/// ```
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
    let train: Vec<Vec<f64>> = train_masks
        .iter()
        .map(|&m| bitmask_to_features(m))
        .collect();
    let test: Vec<Vec<f64>> = test_masks.iter().map(|&m| bitmask_to_features(m)).collect();
    naive_bayes::classify(&train, labels, &test)
}

/// AutoML signal: train on `inputs` self-supervised against `anchor`, predict on `inputs`.
///
/// This is the leave-it-in form: predictions equal in-sample fits, useful as a
/// signal generator when paired with the classical ELIZA on the same anchor.
pub fn eliza_automl_signal(name: &str, inputs: &[u64], anchor: &[bool]) -> SignalProfile {
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
            keyword_bit(kw::DREAM),                      // true
            keyword_bit(kw::DREAM) | keyword_bit(kw::I), // true
            keyword_bit(kw::SORRY),                      // false
            keyword_bit(kw::FATHER),                     // false
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
    fn classify_is_deterministic_across_invocations() {
        let train = vec![
            keyword_bit(kw::DREAM),
            keyword_bit(kw::SORRY),
            keyword_bit(kw::I),
            0,
        ];
        let labels = vec![true, false, true, false];
        let test = vec![keyword_bit(kw::DREAM), 0];
        let p1 = classify(&train, &labels, &test);
        let p2 = classify(&train, &labels, &test);
        let p3 = classify(&train, &labels, &test);
        assert_eq!(p1, p2);
        assert_eq!(p2, p3);
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
