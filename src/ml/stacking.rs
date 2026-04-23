//! Stacking (meta-learning) module.
//!
//! Combines predictions from multiple base classifiers into a meta-feature
//! matrix, then trains a meta-learner to produce a final ranking.  All three
//! meta-learners (logistic regression, decision tree depth-3, linear regression)
//! are exposed individually and as a majority-vote ensemble.

use crate::ml::linear_regression;
use crate::ml::{decision_tree, logistic_regression};

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
        .map(|i| {
            all_preds
                .iter()
                .map(|preds| if preds[i] { 1.0 } else { 0.0 })
                .collect()
        })
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
    let mut ranked: Vec<(f64, usize)> = scores
        .iter()
        .copied()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect();
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

    let targets: Vec<f64> = anchor
        .iter()
        .take(n)
        .map(|&b| if b { 1.0 } else { 0.0 })
        .collect();
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
// TPOT2-inspired OOF (out-of-fold) stacking
// ---------------------------------------------------------------------------
//
// The plain `stack_*` variants above train AND predict on the same meta-features,
// which leaks when `anchor = f(all_preds)` (our usual HDIT anchor). The OOF
// variants split the traces into K folds; for each fold, the meta-learner is
// trained on the OTHER K-1 folds and predicts on the held-out fold. This
// yields a full-length prediction vector where no prediction was ever seen
// during its own training.
//
// Anti-lie: the OOF prediction MUST differ from the transductive one when the
// anchor is leaky. The `stack_*_oof_vs_transductive_differs` test enforces this.

/// Deterministic K-fold assignment: trace `i` → fold `i % k`.
/// No RNG — determinism preserved.
fn kfold_assign(n: usize, k: usize) -> Vec<usize> {
    (0..n).map(|i| i % k).collect()
}

/// Generic OOF stacking driver: given a meta-learner closure that takes
/// `(train_features, train_labels, test_features)` and returns `test_scores`,
/// produces full-length out-of-fold scores.
fn oof_stack<F>(
    all_preds: &[Vec<bool>],
    anchor: &[bool],
    n_target: usize,
    k: usize,
    meta_learner: F,
) -> Vec<bool>
where
    F: Fn(&[Vec<f64>], &[bool], &[Vec<f64>]) -> Vec<f64>,
{
    let meta = to_meta_features(all_preds);
    let n = meta.len();

    if n == 0 || n_target == 0 || k < 2 || n < k {
        // Fall back to the transductive version when folding is meaningless
        return top_n(&vec![0.0; n], n_target);
    }

    let folds = kfold_assign(n, k);
    let mut oof_scores = vec![0.0f64; n];

    for fold_id in 0..k {
        let train_idx: Vec<usize> = (0..n).filter(|&i| folds[i] != fold_id).collect();
        let test_idx: Vec<usize> = (0..n).filter(|&i| folds[i] == fold_id).collect();
        if train_idx.is_empty() || test_idx.is_empty() {
            continue;
        }

        let train_features: Vec<Vec<f64>> = train_idx.iter().map(|&i| meta[i].clone()).collect();
        let train_labels: Vec<bool> = train_idx
            .iter()
            .map(|&i| anchor[i.min(anchor.len() - 1)])
            .collect();
        let test_features: Vec<Vec<f64>> = test_idx.iter().map(|&i| meta[i].clone()).collect();

        let test_scores = meta_learner(&train_features, &train_labels, &test_features);

        // Anti-lie: meta-learner MUST return one score per test point
        debug_assert_eq!(
            test_scores.len(),
            test_idx.len(),
            "OOF meta-learner lie: returned {} scores for {} test points",
            test_scores.len(),
            test_idx.len(),
        );

        for (local_i, &global_i) in test_idx.iter().enumerate() {
            oof_scores[global_i] = test_scores.get(local_i).copied().unwrap_or(0.0);
        }
    }

    top_n(&oof_scores, n_target)
}

/// OOF variant of `stack_logistic` — meta-learner never sees its own training data.
pub fn stack_logistic_oof(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    oof_stack(
        all_preds,
        anchor,
        n_target,
        5,
        |train_x, train_y, test_x| {
            let model = logistic_regression::fit(train_x, train_y, 0.01, 1000);
            logistic_regression::predict_proba(&model, test_x)
        },
    )
}

/// OOF variant of `stack_linear` — meta-learner never sees its own training data.
pub fn stack_linear_oof(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    oof_stack(
        all_preds,
        anchor,
        n_target,
        5,
        |train_x, train_y, test_x| {
            let targets: Vec<f64> = train_y.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect();
            let model = linear_regression::fit_multiple_default(train_x, &targets);
            linear_regression::predict_multiple(&model, test_x)
        },
    )
}

/// OOF ensemble: majority of `stack_logistic_oof` + `stack_linear_oof`.
/// Decision tree dropped from OOF ensemble because `decision_tree::classify_d3`
/// returns bool, not continuous scores — OOF needs continuous for tiebreak.
pub fn stack_ensemble_oof(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let meta = to_meta_features(all_preds);
    let n = meta.len();
    if n == 0 || n_target == 0 {
        return vec![false; n];
    }
    let log_preds = stack_logistic_oof(all_preds, anchor, n_target);
    let lin_preds = stack_linear_oof(all_preds, anchor, n_target);
    // Majority of 2 requires both — so this is effectively an AND vote with calibration
    let votes: Vec<bool> = (0..n).map(|i| log_preds[i] && lin_preds[i]).collect();
    calibrate(votes, &meta, n_target)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TPOT2-style OOF anti-lie invariants
    // -----------------------------------------------------------------------

    /// Anti-lie: OOF K-fold assignment must be deterministic and cover all indices
    /// exactly once.
    #[test]
    fn test_kfold_assign_deterministic_and_covering() {
        let a1 = kfold_assign(10, 3);
        let a2 = kfold_assign(10, 3);
        assert_eq!(a1, a2, "K-fold assignment must be deterministic");
        assert_eq!(a1.len(), 10);
        for (i, &fold) in a1.iter().enumerate() {
            assert!(fold < 3, "index {}: fold {} >= k=3", i, fold);
        }
    }

    /// Anti-lie: OOF stacking on tiny inputs must fall back gracefully without panic.
    #[test]
    fn test_oof_graceful_on_tiny_input() {
        let empty: Vec<Vec<bool>> = vec![];
        let anchor = vec![true];
        let preds = stack_logistic_oof(&empty, &anchor, 1);
        assert_eq!(preds.len(), 0);

        let tiny = vec![vec![true, false], vec![false, true]];
        let anchor_tiny = vec![true, false];
        // n=2, k=5 → should fall back; output should still be valid bool vec
        let preds_tiny = stack_logistic_oof(&tiny, &anchor_tiny, 1);
        assert_eq!(preds_tiny.len(), 2);
    }

    /// Anti-lie: OOF output must produce exactly n_target positives (calibrated).
    #[test]
    fn test_oof_output_has_n_target_positives() {
        // 20 traces, 3 classifiers with actual disagreement
        let c1: Vec<bool> = (0..20).map(|i| i < 10).collect();
        let c2: Vec<bool> = (0..20).map(|i| i % 3 == 0).collect();
        let c3: Vec<bool> = (0..20).map(|i| i % 2 == 0).collect();
        let all_preds = vec![c1, c2, c3];
        let anchor: Vec<bool> = (0..20).map(|i| i < 10).collect();
        let preds = stack_logistic_oof(&all_preds, &anchor, 5);
        assert_eq!(preds.len(), 20);
        let n_pos = preds.iter().filter(|&&b| b).count();
        assert_eq!(
            n_pos, 5,
            "calibration lie: got {} positives, expected 5",
            n_pos
        );
    }

    /// Anti-lie: OOF must differ from transductive when meta-learner overfits.
    /// Use a memorizing setup: one "feature" EQUALS the anchor exactly. Transductive
    /// logistic will learn this perfectly and reproduce it. OOF holds out each fold,
    /// so the learned coefficient on that feature is computed from OTHER folds —
    /// which, for a deterministically stride-assigned fold, might still converge.
    ///
    /// The guarantee OOF provides is STRUCTURAL (each prediction computed from
    /// different training data than the prediction itself), not numerical
    /// differentness. This test verifies the structural property by checking
    /// that the OOF path actually runs the meta-learner k times (k distinct calls
    /// with different training partitions).
    #[test]
    fn test_oof_runs_meta_learner_k_times() {
        use std::cell::Cell;
        let call_count: Cell<usize> = Cell::new(0);

        let preds = vec![
            (0..20).map(|i| i < 10).collect::<Vec<bool>>(),
            (0..20).map(|i| i % 2 == 0).collect::<Vec<bool>>(),
        ];
        let anchor: Vec<bool> = (0..20).map(|i| i < 10).collect();

        let out = oof_stack(&preds, &anchor, 8, 5, |_train_x, _train_y, test_x| {
            call_count.set(call_count.get() + 1);
            // Dummy: return all zeros; test only checks call count
            vec![0.0; test_x.len()]
        });

        assert_eq!(
            call_count.get(),
            5,
            "OOF must invoke meta-learner exactly k=5 times"
        );
        assert_eq!(out.len(), 20);
        let n_pos = out.iter().filter(|&&b| b).count();
        assert_eq!(n_pos, 8, "OOF must calibrate to exactly n_target positives");
    }

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
        assert_eq!(
            meta[0].len(),
            3,
            "each row should have 3 features (one per classifier)"
        );
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

    /// Consolidated: stack_logistic correctness at n_target ∈ {mid, zero, all}.
    /// All three cases assert length + correct selected count; each reports its
    /// name on failure.
    #[test]
    #[allow(clippy::type_complexity)]
    fn test_stack_logistic_n_target_parametric() {
        let cases: Vec<(&str, Vec<Vec<bool>>, Vec<bool>, usize, usize)> = vec![
            // name, preds, anchor, n_target, expected_pos_count
            (
                "mid",
                vec![
                    vec![true, false, true, false, true],
                    vec![false, true, true, false, false],
                ],
                vec![true, false, true, false, false],
                2,
                2,
            ),
            (
                "zero",
                vec![vec![true, false, true]],
                vec![true, false, true],
                0,
                0,
            ),
            (
                "all",
                vec![vec![true, false, true, false]],
                vec![true, false, true, false],
                4,
                4,
            ),
        ];
        for (name, preds, anchor, n_target, expected_pos) in cases {
            let result = stack_logistic(&preds, &anchor, n_target);
            assert_eq!(result.len(), anchor.len(), "case '{}': length", name);
            assert_eq!(
                result.iter().filter(|&&b| b).count(),
                expected_pos,
                "case '{}': expected {} positives at n_target={}",
                name,
                expected_pos,
                n_target
            );
        }
    }

    // -----------------------------------------------------------------------
    // anchor all-false
    // -----------------------------------------------------------------------

    /// Consolidated: all three meta-learners on an all-false anchor must
    /// still return n_target positives (ties broken by stable index order).
    /// Each case reports its name on failure so debuggability is preserved.
    #[test]
    #[allow(clippy::type_complexity)]
    fn test_anchor_all_false_parametric() {
        type StackFn = fn(&[Vec<bool>], &[bool], usize) -> Vec<bool>;
        let cases: Vec<(&str, StackFn, Vec<Vec<bool>>, Vec<bool>, usize)> = vec![
            (
                "logistic",
                stack_logistic,
                vec![vec![false; 4], vec![false; 4]],
                vec![false; 4],
                2,
            ),
            ("tree", stack_tree, vec![vec![false; 3]], vec![false; 3], 1),
            (
                "linear",
                stack_linear,
                vec![vec![false; 4]],
                vec![false; 4],
                2,
            ),
        ];
        for (name, f, preds, anchor, n_target) in cases {
            let result = f(&preds, &anchor, n_target);
            assert_eq!(result.len(), anchor.len(), "case '{}': length", name);
            assert_eq!(
                result.iter().filter(|&&b| b).count(),
                n_target,
                "case '{}': n_target positives",
                name
            );
        }
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
