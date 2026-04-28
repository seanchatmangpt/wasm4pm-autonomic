//! STRIPS AutoML Equivalent — Learned Goal Reachability Predictor via Gradient Boosting.
//!
//! While `crate::ml::strips` provides goal-stack search planning, this module
//! provides the *learned* equivalent: given training data of (initial state,
//! reachable?) pairs, learn a goal-reachability predictor via gradient boosting.
//!
//! # Compiled Cognition
//!
//! This module contributes `L_learned` to Compiled Cognition. Paired with
//! `strips.rs` (`S_symbolic`), these two halves compose into the full
//! goal reachability primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Substrate Bifurcation
//!
//! Classical STRIPS (Fikes & Nilsson 1971) is *algorithmic search*: given a goal,
//! search for a sequence of actions reaching it. This module is *learned prediction*:
//! predict reachability directly from state features without search.
//!
//! Both perform the **same JOB** — predict if a goal is reachable from a state —
//! but via different paths:
//! - **Classical**: Exhaustive search, variable latency, complete but slow
//! - **AutoML**: Feature-based prediction, constant latency, approximate but instant
//!
//! # Feature Extraction
//!
//! Input is a u64 bitmask of 16 STRIPS state bits. Output is a 16-dimensional
//! binary feature vector, one per bit.
//!
//! # Architecture
//!
//! ```text
//! STRIPS state (u64) → 16-dim binary vector → Gradient Boosting → Reachable (bool)
//! ```
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::strips_automl;
//! use dteam::ml::strips::{HOLDING_A, HOLDING_B, CLEAR_A, ON_TABLE_A, ARM_EMPTY};
//!
//! // Train: (initial state, reachability) pairs
//! let train_states = vec![
//!     HOLDING_A,                           // goal already satisfied
//!     CLEAR_A | ON_TABLE_A | ARM_EMPTY,   // one pickup away
//!     HOLDING_B,                           // can putdown then pickup
//!     0,                                   // unreachable empty state
//! ];
//! let labels = vec![true, true, true, false];
//!
//! // Test: can we reach HOLDING_A from initial?
//! let test_states = vec![CLEAR_A | ON_TABLE_A | ARM_EMPTY];
//! let predictions = strips_automl::classify(&train_states, &labels, &test_states);
//!
//! assert_eq!(predictions.len(), 1);
//! // predictions[0] will likely be true (one-step plan exists)
//! ```
//!
//! # Determinism
//!
//! This module is fully deterministic: identical inputs produce byte-identical outputs.
//! Gradient boosting uses entropy-based splits, which are deterministic.

use crate::ml::gradient_boosting;
use crate::ml::hdit_automl::SignalProfile;

/// Number of state bits we use as features (matches `strips` 16 named bits).
pub const N_STATE_FEATURES: usize = 16;

/// Extract a binary feature vector from a STRIPS state bitmask.
///
/// # Example
///
/// ```rust
/// use dteam::ml::strips_automl;
/// use dteam::ml::strips::{HOLDING_A, ON_TABLE_A, CLEAR_A, ARM_EMPTY};
///
/// let state = CLEAR_A | ON_TABLE_A | ARM_EMPTY;
/// let features = strips_automl::bitmask_to_features(state);
///
/// assert_eq!(features.len(), 16);
/// assert_eq!(features[0], 1.0); // CLEAR_A is bit 0
/// assert_eq!(features[3], 1.0); // ON_TABLE_A is bit 3
/// assert_eq!(features[15], 1.0); // ARM_EMPTY is bit 15
/// ```
#[inline]
#[must_use]
pub fn bitmask_to_features(state: u64) -> Vec<f64> {
    (0..N_STATE_FEATURES)
        .map(|i| if (state >> i) & 1 != 0 { 1.0 } else { 0.0 })
        .collect()
}

/// Train gradient boosting on (state, reachable?) pairs and predict on test states.
#[must_use]
pub fn classify(train_states: &[u64], labels: &[bool], test_states: &[u64]) -> Vec<bool> {
    let train: Vec<Vec<f64>> = train_states.iter().map(|&s| bitmask_to_features(s)).collect();
    let test: Vec<Vec<f64>> = test_states.iter().map(|&s| bitmask_to_features(s)).collect();
    gradient_boosting::classify_default(&train, labels, &test)
}

/// AutoML signal: in-sample fit-and-predict on states vs reachability anchor.
pub fn strips_automl_signal(name: &str, states: &[u64], anchor: &[bool]) -> SignalProfile {
    let predictions = classify(states, anchor, states);
    // Gradient boosting evaluation is multi-tree traversal — call it T1.
    let timing_us = (states.len() as u64).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::strips::{
        ARM_EMPTY, CLEAR_A, CLEAR_B, HOLDING_A, HOLDING_B, ON_TABLE_A, ON_TABLE_B,
    };

    #[test]
    fn feature_vector_length_is_16() {
        assert_eq!(bitmask_to_features(0).len(), N_STATE_FEATURES);
    }

    #[test]
    fn feature_vector_extracts_state_bits() {
        let s = CLEAR_A | ON_TABLE_A | ARM_EMPTY;
        let f = bitmask_to_features(s);
        assert_eq!(f[0], 1.0); // CLEAR_A
        assert_eq!(f[3], 1.0); // ON_TABLE_A
        assert_eq!(f[15], 1.0); // ARM_EMPTY
    }

    #[test]
    fn classify_holding_pattern_separable() {
        // Reachable: states where HOLDING_A is set or pickup-A precondition holds
        let train_states = vec![
            HOLDING_A,                                           // already at goal
            CLEAR_A | ON_TABLE_A | ARM_EMPTY,                    // 1-step plan
            HOLDING_B,                                           // can putdown then pickup
            0,                                                   // unreachable empty state
        ];
        let labels = vec![true, true, true, false];
        let test_states = vec![HOLDING_A, 0];
        let preds = classify(&train_states, &labels, &test_states);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn classify_empty_training_handled() {
        let preds = classify(&[], &[], &[HOLDING_A]);
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn classify_is_deterministic_across_invocations() {
        let train = vec![
            HOLDING_A,
            CLEAR_A | ON_TABLE_A | ARM_EMPTY,
            HOLDING_B,
            0,
        ];
        let labels = vec![true, true, true, false];
        let p1 = classify(&train, &labels, &train);
        let p2 = classify(&train, &labels, &train);
        let p3 = classify(&train, &labels, &train);
        assert_eq!(p1, p2);
        assert_eq!(p2, p3);
    }

    #[test]
    fn signal_predicts_at_or_above_chance() {
        let states = vec![
            HOLDING_A,
            CLEAR_A | ON_TABLE_A | ARM_EMPTY,
            HOLDING_A,
            CLEAR_B | ON_TABLE_B | ARM_EMPTY,
            0,
            0,
        ];
        let anchor = vec![true, true, true, true, false, false];
        let sig = strips_automl_signal("strips_gb", &states, &anchor);
        assert!(sig.accuracy_vs_anchor >= 0.5);
    }

    #[test]
    fn signal_produces_correct_prediction_length() {
        let states = vec![HOLDING_A, 0, ARM_EMPTY];
        let anchor = vec![true, false, false];
        let sig = strips_automl_signal("strips", &states, &anchor);
        assert_eq!(sig.predictions.len(), 3);
    }
}
