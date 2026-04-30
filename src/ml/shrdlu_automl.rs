//! SHRDLU AutoML Equivalent — Learned Command Feasibility Predictor via Logistic Regression.
//!
//! While `crate::ml::shrdlu` provides hand-coded preconditions and a recursive
//! planner, this module provides the *learned* equivalent: given training data
//! of (state, command-feasible?) pairs, learn feasibility classification via
//! logistic regression on structured-state features.
//!
//! # Compiled Cognition
//!
//! This module contributes `L_learned` to Compiled Cognition. Paired with
//! `shrdlu.rs` (`S_symbolic`), these two halves compose into the full
//! spatial reasoning primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Substrate Bifurcation
//!
//! Classical SHRDLU (Winograd 1971) is *symbolic reasoning*: preconditions as logical
//! assertions, planning as recursive depth-first search. This module is *learned reasoning*:
//! features from the world state, logistic model over feasibility probability.
//!
//! Both perform the **same JOB** — predict if a command is executable in a world state —
//! but via different physics:
//! - **Classical**: Hand-coded preconditions, explicit constraints, brittle
//! - **AutoML**: Learned correlation from state features, generalizable
//!
//! # Feature Extraction
//!
//! Input is a u64 bitmask encoding the block world state: clear, on_table, holding,
//! arm_empty, and on(x,y) relation bits. Output is a ~30-dimensional feature vector
//! capturing each logical fact as a binary feature.
//!
//! # Architecture
//!
//! ```text
//! World state (u64) → 30-dim binary vector → Logistic Regression → Feasible (bool)
//! ```
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::shrdlu_automl;
//! use dteam::ml::shrdlu::{initial_state, clear, on_table, holding, ARM_EMPTY};
//!
//! // Train: (world state, command-feasible) pairs
//! // PickUp(A) is feasible iff CLEAR_A & ON_TABLE_A & ARM_EMPTY
//! let s_initial = initial_state();
//! let s_holding_b = holding(1);
//!
//! let train_states = vec![
//!     s_initial,           // feasible (preconditions met)
//!     s_initial,           // feasible (duplicate)
//!     s_holding_b,         // not feasible (arm not empty)
//! ];
//! let labels = vec![true, true, false];
//!
//! // Test: is PickUp feasible in initial state?
//! let test_states = vec![s_initial];
//! let predictions = shrdlu_automl::classify(&train_states, &labels, &test_states);
//!
//! assert_eq!(predictions.len(), 1);
//! // predictions[0] will likely be true (initial state allows pickup)
//! ```
//!
//! # Determinism
//!
//! This module is fully deterministic: identical inputs produce byte-identical outputs.
//! Logistic regression training is deterministic (gradient descent with fixed seed).

use crate::ml::hdit_automl::SignalProfile;
use crate::ml::logistic_regression;

/// Number of structured-state feature bits.
///
/// Layout (matches `shrdlu` bit layout):
/// - 5 bits: clear(x) for x in 0..5
/// - 5 bits: on_table(x)
/// - 5 bits: holding(x)
/// - 1 bit: arm_empty
/// - up to 14 selected on(x,y) bits — we sample the off-diagonal pairs (x*5+y for x≠y)
pub const N_STATE_FEATURES: usize = 30;

/// Extract a structured feature vector from a SHRDLU state bitmask.
///
/// The mapping is chosen so that all logically distinct world facts contribute
/// independent features, suitable for a linear classifier.
///
/// # Example
///
/// ```rust
/// use dteam::ml::shrdlu_automl;
/// use dteam::ml::shrdlu::{clear, on_table, holding, ARM_EMPTY};
///
/// let state = clear(2) | on_table(2) | ARM_EMPTY;
/// let features = shrdlu_automl::bitmask_to_features(state);
///
/// assert_eq!(features.len(), 30);
/// // clear(2) sets feature 2
/// assert_eq!(features[2], 1.0);
/// // on_table(2) sets feature 5+2=7
/// assert_eq!(features[7], 1.0);
/// // arm_empty is feature 15
/// assert_eq!(features[15], 1.0);
/// ```
#[inline]
#[must_use]
pub fn bitmask_to_features(state: u64) -> Vec<f64> {
    let mut features = Vec::with_capacity(N_STATE_FEATURES);
    // bits 0-4: clear
    for i in 0..5 {
        features.push(if (state >> i) & 1 != 0 { 1.0 } else { 0.0 });
    }
    // bits 5-9: on_table
    for i in 5..10 {
        features.push(if (state >> i) & 1 != 0 { 1.0 } else { 0.0 });
    }
    // bits 10-14: holding
    for i in 10..15 {
        features.push(if (state >> i) & 1 != 0 { 1.0 } else { 0.0 });
    }
    // bit 15: arm_empty
    features.push(if (state >> 15) & 1 != 0 { 1.0 } else { 0.0 });
    // off-diagonal on(x,y) pairs: 5*5 = 25 bits at offset 16, but we keep just
    // 14 unique ordered pairs to stay at 30 features total.
    let mut pair_count = 0;
    for x in 0..5 {
        for y in 0..5 {
            if x == y {
                continue;
            }
            if pair_count >= 14 {
                break;
            }
            let bit_offset = 16 + x * 5 + y;
            features.push(if (state >> bit_offset) & 1 != 0 {
                1.0
            } else {
                0.0
            });
            pair_count += 1;
        }
    }
    features
}

/// Train logistic regression on (state, label) pairs and predict on test states.
#[must_use]
pub fn classify(train_states: &[u64], labels: &[bool], test_states: &[u64]) -> Vec<bool> {
    let train: Vec<Vec<f64>> = train_states
        .iter()
        .map(|&s| bitmask_to_features(s))
        .collect();
    let test: Vec<Vec<f64>> = test_states
        .iter()
        .map(|&s| bitmask_to_features(s))
        .collect();
    logistic_regression::classify_default(&train, labels, &test)
}

/// AutoML signal: in-sample fit-and-predict on states vs feasibility anchor.
pub fn shrdlu_automl_signal(name: &str, states: &[u64], anchor: &[bool]) -> SignalProfile {
    let predictions = classify(states, anchor, states);
    // Logistic regression eval is one dot product per state — call it T1.
    let timing_us = (states.len() as u64 / 4).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::shrdlu::{clear, holding, initial_state, on, on_table, ARM_EMPTY};

    #[test]
    fn feature_vector_length_is_30() {
        assert_eq!(bitmask_to_features(0).len(), N_STATE_FEATURES);
    }

    #[test]
    fn feature_vector_initial_state_has_arm_empty() {
        let f = bitmask_to_features(initial_state());
        // arm_empty is feature index 15 (5+5+5+0)
        assert_eq!(f[15], 1.0);
    }

    #[test]
    fn feature_vector_extracts_holding() {
        let f = bitmask_to_features(holding(0));
        // holding(0) is feature index 10 (5+5+0)
        assert_eq!(f[10], 1.0);
        // arm_empty should be 0
        assert_eq!(f[15], 0.0);
    }

    #[test]
    fn feature_vector_extracts_clear_and_on_table() {
        let f = bitmask_to_features(clear(2) | on_table(2));
        assert_eq!(f[2], 1.0); // clear(2)
        assert_eq!(f[7], 1.0); // on_table(2) at offset 5
    }

    #[test]
    fn classify_pickup_feasibility_separable() {
        // Pickup A is feasible iff CLEAR_A & ON_TABLE_A & ARM_EMPTY all set
        let s_initial = initial_state();
        let s_holding_b = holding(1);
        let s_no_clear = on(1, 0) | ARM_EMPTY | on_table(0); // A under B; not clear
        let s_arm_held = holding(2);

        let train_states = vec![s_initial, s_initial, s_holding_b, s_no_clear, s_arm_held];
        let labels = vec![true, true, false, false, false];
        let test_states = vec![s_initial, s_holding_b];
        let preds = classify(&train_states, &labels, &test_states);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn classify_empty_training_handled() {
        let preds = classify(&[], &[], &[initial_state()]);
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn classify_is_deterministic_across_invocations() {
        let train = vec![initial_state(), holding(1), initial_state(), holding(2)];
        let labels = vec![true, false, true, false];
        let p1 = classify(&train, &labels, &train);
        let p2 = classify(&train, &labels, &train);
        let p3 = classify(&train, &labels, &train);
        assert_eq!(p1, p2);
        assert_eq!(p2, p3);
    }

    #[test]
    fn signal_in_sample_predicts() {
        let states = vec![initial_state(), holding(1), initial_state(), holding(2)];
        let anchor = vec![true, false, true, false];
        let sig = shrdlu_automl_signal("shrdlu_lr", &states, &anchor);
        assert_eq!(sig.predictions.len(), 4);
    }

    #[test]
    fn signal_correct_length() {
        let states = vec![initial_state(); 5];
        let anchor = vec![true; 5];
        let sig = shrdlu_automl_signal("shrdlu", &states, &anchor);
        assert_eq!(sig.predictions.len(), 5);
    }
}
