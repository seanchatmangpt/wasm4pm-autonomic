//! Stacking (meta-learning) module.
//!
//! Combines predictions from multiple base classifiers into a meta-feature
//! matrix, then trains a meta-learner to produce a final ranking.  All three
//! meta-learners (logistic regression, decision tree depth-3, linear regression)
//! are exposed individually and as a majority-vote ensemble.

use crate::ml::{decision_tree, logistic_regression};
use crate::ml::linear_regression;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Convert `all_preds` (k classifiers × n traces) into a meta-feature matrix.
///
/// `meta_features[i] = [preds[0][i] as f64, preds[1][i] as f64, ..., preds[k-1][i] as f64]`
///
/// Returns an empty `Vec` when `all_preds` is empty.  If classifiers have
/// different lengths the shortest one determines `n`.
fn to_meta_features(all_preds: &[Vec<bool>]) -> Vec<Vec<f64>> {
    if all_preds.is_empty() {
        return vec![];
    }

    let n = all_preds.iter().map(|v| v.len()).min().unwrap_or(0);
    (0..n)
        .map(|i| all_preds.iter().map(|preds| if preds[i] { 1.0 } else { 0.0 }).collect())
        .collect()
}

/// Given a slice of f64 scores and `n_target`, return a bool mask where the
/// `n_target` highest-scoring indices are `true`.
///
/// Ties are broken by stable index ordering (lower index wins).
fn top_n(scores: &[f64], n_target: usize) -> Vec<bool> {
    let n = scores.len();
    if n_target == 0 {
        return vec![false; n];
    }
    if n_target >= n {
        return vec![true; n];
    }

    // Collect (score, index), sort descending by score (stable by index on ties).
    let mut ranked: Vec<(f64, usize)> = scores.iter().copied().enumerate().map(|(i, s)| (s, i)).collect();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut result = vec![false; n];
    for &(_, idx) in ranked.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

/// Calibrate a `Vec<bool>` (possibly wrong count) to exactly `n_target` trues.
///
/// If `count == n_target` return as-is.
/// Otherwise rank by `meta_feature` row-sum descending and take the top `n_target`.
fn calibrate(preds: Vec<bool>, meta: &[Vec<f64>], n_target: usize) -> Vec<bool> {
    let n = preds.len();
    let current = preds.iter().filter(|&&b| b).count();
    if current == n_target {
        return preds;
    }

    // Fall back to ranking by meta-feature sum.
    let scores: Vec<f64> = (0..n)
        .map(|i| meta.get(i).map_or(0.0, |row| row.iter().sum::<f64>()))
        .collect();
    top_n(&scores, n_target)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Stack classify using logistic regression as meta-learner.
///
/// Trains logistic regression on meta-features with `anchor` as labels.
/// Predicts on the same meta-features (transductive).
/// Calibrates to `n_target` by ranking predicted probabilities.
pub fn stack_logistic(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let meta = to_meta_features(all_preds);
    let n = meta.len();

    if n == 0 || n_target == 0 {
        return vec![false; n];
    }

    let train_labels: Vec<bool> = anchor.iter().take(n).copied().collect();
    let model = logistic_regression::fit(&meta, &train_labels, 0.01, 1000);
    let probas = logistic_regression::predict_proba(&model, &meta);
    top_n(&probas, n_target)
}

/// Stack classify using decision tree (depth 3) as meta-learner.
///
/// If the raw tree prediction count differs from `n_target`, calibrates by
/// ranking on meta-feature row-sum.
pub fn stack_tree(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let meta = to_meta_features(all_preds);
    let n = meta.len();

    if n == 0 || n_target == 0 {
        return vec![false; n];
    }

    let train_labels: Vec<bool> = anchor.iter().take(n).copied().collect();
    let raw = decision_tree::classify_d3(&meta, &train_labels, &meta);
    calibrate(raw, &meta, n_target)
}

/// Stack classify using linear regression as meta-learner.
///
/// Trains on meta-features with `anchor` as 0/1 targets, ranks test traces by
/// predicted value, and takes the top `n_target`.
pub fn stack_linear(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let meta = to_meta_features(all_preds);
    let n = meta.len();

    if n == 0 || n_target == 0 {
        return vec![false; n];
    }

    let targets: Vec<f64> = anchor.iter().take(n).map(|&b| if b { 1.0 } else { 0.0 }).collect();
    let model = linear_regression::fit_multiple_default(&meta, &targets);
    let scores = linear_regression::predict_multiple(&model, &meta);
    top_n(&scores, n_target)
}

/// Ensemble of all three meta-learners (logistic, tree, linear) — majority vote.
///
/// Each meta-learner casts a `bool` vote per trace; a trace is selected when
/// at least 2 of 3 votes are `true`.  The result is then calibrated to exactly
/// `n_target` by ranking on meta-feature row-sum if the vote count diverges.
pub fn stack_ensemble(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let meta = to_meta_features(all_preds);
    let n = meta.len();

    if n == 0 || n_target == 0 {
        return vec![false; n];
    }

    let log_preds = stack_logistic(all_preds, anchor, n_target);
    let tree_preds = stack_tree(all_preds, anchor, n_target);
    let lin_preds = stack_linear(all_preds, anchor, n_target);

    // Majority vote (≥2 of 3).
    let votes: Vec<bool> = (0..n)
        .map(|i| {
            let v = log_preds[i] as usize + tree_preds[i] as usize + lin_preds[i] as usize;
            v >= 2
        })
        .collect();

    calibrate(votes, &meta, n_target)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // to_meta_features
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_meta_features_shape() {
        // 3 classifiers, 4 traces each.
        let all_preds = vec![
            vec![true, false, true, false],
            vec![false, false, true, true],
            vec![true, true, false, false],
        ];
        let meta = to_meta_features(&all_preds);
        assert_eq!(meta.len(), 4, "should have 4 rows (one per trace)");
        assert_eq!(meta[0].len(), 3, "each row should have 3 features (one per classifier)");
        // Row 0: [1.0, 0.0, 1.0]
        assert_eq!(meta[0], vec![1.0, 0.0, 1.0]);
        // Row 1: [0.0, 0.0, 1.0]
        assert_eq!(meta[1], vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_to_meta_features_empty() {
        let meta = to_meta_features(&[]);
        assert!(meta.is_empty());
    }

    #[test]
    fn test_to_meta_features_single_classifier() {
        let all_preds = vec![vec![true, false, true]];
        let meta = to_meta_features(&all_preds);
        assert_eq!(meta.len(), 3);
        assert_eq!(meta[0], vec![1.0]);
        assert_eq!(meta[1], vec![0.0]);
        assert_eq!(meta[2], vec![1.0]);
    }

    // -----------------------------------------------------------------------
    // stack_logistic
    // -----------------------------------------------------------------------

    #[test]
    fn test_stack_logistic_returns_correct_length() {
        let all_preds = vec![
            vec![true, false, true, false, true],
            vec![false, true, true, false, false],
        ];
        let anchor = vec![true, false, true, false, false];
        let n_target = 2;

        let result = stack_logistic(&all_preds, &anchor, n_target);
        assert_eq!(result.len(), 5, "result length should equal number of traces");
        let selected = result.iter().filter(|&&b| b).count();
        assert_eq!(selected, n_target, "exactly n_target traces should be selected");
    }

    #[test]
    fn test_stack_logistic_n_target_zero() {
        let all_preds = vec![vec![true, false, true]];
        let anchor = vec![true, false, true];
        let result = stack_logistic(&all_preds, &anchor, 0);
        assert!(result.iter().all(|&b| !b), "n_target=0 should select nothing");
    }

    #[test]
    fn test_stack_logistic_n_target_all() {
        let all_preds = vec![vec![true, false, true, false]];
        let anchor = vec![true, false, true, false];
        let result = stack_logistic(&all_preds, &anchor, 4);
        assert_eq!(result.iter().filter(|&&b| b).count(), 4);
    }

    // -----------------------------------------------------------------------
    // anchor all-false
    // -----------------------------------------------------------------------

    #[test]
    fn test_anchor_all_false_logistic() {
        // All anchor labels are false → gradient descent converges to all-false
        // predictions.  stack_logistic should still return a Vec of length n
        // and exactly n_target selected (top-n by probability, which will be
        // near-uniform — ties broken by index).
        let all_preds = vec![
            vec![false, false, false, false],
            vec![false, false, false, false],
        ];
        let anchor = vec![false, false, false, false];
        let n_target = 2;

        let result = stack_logistic(&all_preds, &anchor, n_target);
        assert_eq!(result.len(), 4);
        assert_eq!(result.iter().filter(|&&b| b).count(), n_target);
    }

    #[test]
    fn test_anchor_all_false_tree() {
        let all_preds = vec![vec![false, false, false]];
        let anchor = vec![false, false, false];

        let result = stack_tree(&all_preds, &anchor, 1);
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().filter(|&&b| b).count(), 1);
    }

    #[test]
    fn test_anchor_all_false_linear() {
        let all_preds = vec![vec![false, false, false, false]];
        let anchor = vec![false, false, false, false];

        let result = stack_linear(&all_preds, &anchor, 2);
        assert_eq!(result.len(), 4);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }

    // -----------------------------------------------------------------------
    // stack_ensemble
    // -----------------------------------------------------------------------

    #[test]
    fn test_stack_ensemble_correct_length_and_count() {
        let all_preds = vec![
            vec![true, false, true, false, true, false],
            vec![true, true, false, false, true, false],
            vec![false, true, true, false, false, true],
        ];
        let anchor = vec![true, true, false, false, true, false];
        let n_target = 3;

        let result = stack_ensemble(&all_preds, &anchor, n_target);
        assert_eq!(result.len(), 6);
        assert_eq!(
            result.iter().filter(|&&b| b).count(),
            n_target,
            "ensemble must select exactly n_target traces"
        );
    }

    #[test]
    fn test_stack_ensemble_empty_preds() {
        let result = stack_ensemble(&[], &[], 2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_stack_ensemble_n_target_zero() {
        let all_preds = vec![vec![true, false, true]];
        let anchor = vec![true, false, true];
        let result = stack_ensemble(&all_preds, &anchor, 0);
        assert!(result.iter().all(|&b| !b));
    }

    // -----------------------------------------------------------------------
    // stack_tree
    // -----------------------------------------------------------------------

    #[test]
    fn test_stack_tree_correct_count() {
        let all_preds = vec![
            vec![true, false, true, false],
            vec![false, true, true, false],
        ];
        let anchor = vec![true, false, true, false];

        let result = stack_tree(&all_preds, &anchor, 2);
        assert_eq!(result.len(), 4);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }

    // -----------------------------------------------------------------------
    // stack_linear
    // -----------------------------------------------------------------------

    #[test]
    fn test_stack_linear_correct_count() {
        let all_preds = vec![
            vec![true, false, true, false, true],
            vec![false, false, true, true, false],
        ];
        let anchor = vec![true, false, true, false, false];

        let result = stack_linear(&all_preds, &anchor, 3);
        assert_eq!(result.len(), 5);
        assert_eq!(result.iter().filter(|&&b| b).count(), 3);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_all_preds() {
        assert!(stack_logistic(&[], &[], 1).is_empty());
        assert!(stack_tree(&[], &[], 1).is_empty());
        assert!(stack_linear(&[], &[], 1).is_empty());
        assert!(stack_ensemble(&[], &[], 1).is_empty());
    }

    #[test]
    fn test_top_n_ties_stable() {
        // All scores equal → first n_target indices should be selected.
        let scores = vec![1.0, 1.0, 1.0, 1.0];
        let result = top_n(&scores, 2);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }
}
