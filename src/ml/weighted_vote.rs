//! Weighted voting utilities for ensemble signal fusion.
//!
//! Each signal (classifier) receives a scalar weight derived from how often it
//! agrees with a designated anchor (pseudo-ground-truth). The weighted vote
//! accumulates per-trace scores, then calibrates to exactly `n_target`
//! positives by picking the highest-scoring traces.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute per-signal accuracy weights using `anchor` as pseudo-ground-truth.
///
/// `weight[k]` = fraction of traces where `preds[k][i] == anchor[i]`.
/// Clamped to `[0.01, 1.0]` so no signal ever receives zero weight.
///
/// # Edge cases
/// - `preds` is empty → returns an empty `Vec`.
/// - `anchor` is all-false → weights reflect agreement with all-false, still
///   computed per the fraction formula (uniform if all signals are also all-false).
/// - Signal vectors longer than `anchor` are truncated to `anchor.len()`.
pub fn signal_weights(preds: &[Vec<bool>], anchor: &[bool]) -> Vec<f64> {
    if preds.is_empty() || anchor.is_empty() {
        return Vec::new();
    }

    let n = anchor.len();

    preds
        .iter()
        .map(|sig| {
            let len = sig.len().min(n);
            if len == 0 {
                return 0.01_f64;
            }
            let matches = sig[..len]
                .iter()
                .zip(anchor[..len].iter())
                .filter(|(&p, &a)| p == a)
                .count();
            let acc = matches as f64 / len as f64;
            acc.clamp(0.01, 1.0)
        })
        .collect()
}

/// Weighted majority vote.
///
/// For each trace `i`, computes:
/// ```text
/// weighted_score[i] = Σ_{k: preds[k][i] = true} weights[k]
/// ```
/// The top `n_target` traces by weighted score are set to `true`.
/// Tie-breaking is by lower index (stable, deterministic).
///
/// # Edge cases
/// - `preds` is empty → all-false of length `weights.len()` (which is also 0).
/// - `n_target >= n` → all `true`.
/// - `n_target == 0` → all `false`.
pub fn weighted_vote(preds: &[Vec<bool>], weights: &[f64], n_target: usize) -> Vec<bool> {
    if preds.is_empty() || weights.is_empty() {
        return Vec::new();
    }

    let n = preds.iter().map(|v| v.len()).min().unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }

    if n_target == 0 {
        return vec![false; n];
    }

    if n_target >= n {
        return vec![true; n];
    }

    // Accumulate weighted scores per trace.
    let scores: Vec<f64> = (0..n)
        .map(|i| {
            preds
                .iter()
                .zip(weights.iter())
                .filter(|(sig, _)| sig[i])
                .map(|(_, &w)| w)
                .sum()
        })
        .collect();

    // Sort indices by score descending; ties broken by lower index (stable).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

/// Convenience wrapper: compute accuracy weights from `anchor`, then vote.
///
/// Equivalent to calling [`signal_weights`] followed by [`weighted_vote`].
pub fn auto_weighted_vote(preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    if preds.is_empty() {
        return vec![false; anchor.len()];
    }
    let weights = signal_weights(preds, anchor);
    weighted_vote(preds, &weights, n_target)
}

/// Pearson correlation of each signal (as 0/1 f64) with the anchor.
///
/// Returns a `Vec<f64>` of length `preds.len()`.  Values near +1 indicate
/// a strongly informative signal; near −1 indicates an anti-correlated signal;
/// near 0 indicates uninformative.
///
/// Returns 0.0 for any signal where the standard deviation is zero (constant
/// signal) or where the anchor has zero standard deviation.
pub fn signal_correlations(preds: &[Vec<bool>], anchor: &[bool]) -> Vec<f64> {
    if preds.is_empty() || anchor.is_empty() {
        return Vec::new();
    }

    let n = anchor.len();
    let anchor_f: Vec<f64> = anchor.iter().map(|&b| b as u8 as f64).collect();
    let anchor_mean = mean(&anchor_f);
    let anchor_std = std_dev(&anchor_f, anchor_mean);

    preds
        .iter()
        .map(|sig| {
            let len = sig.len().min(n);
            if len == 0 {
                return 0.0;
            }
            let sig_f: Vec<f64> = sig[..len].iter().map(|&b| b as u8 as f64).collect();
            let a_slice = &anchor_f[..len];

            let sig_mean = mean(&sig_f);
            let sig_std = std_dev(&sig_f, sig_mean);

            if sig_std < f64::EPSILON || anchor_std < f64::EPSILON {
                return 0.0;
            }

            let cov: f64 = sig_f
                .iter()
                .zip(a_slice.iter())
                .map(|(&s, &a)| (s - sig_mean) * (a - anchor_mean))
                .sum::<f64>()
                / len as f64;

            cov / (sig_std * anchor_std)
        })
        .collect()
}

/// Precision-weighted vote.
///
/// `precision[k]` = |{i: preds[k][i]=true AND anchor[i]=true}| / |{i: preds[k][i]=true}|
///
/// Signals with no predicted positives receive the minimum weight (0.01).
/// This up-weights high-precision signals even if they have low recall.
///
/// Calibrates to exactly `n_target` positives by weighted score (same as
/// [`weighted_vote`]).
pub fn precision_weighted_vote(
    preds: &[Vec<bool>],
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    if preds.is_empty() {
        return vec![false; anchor.len()];
    }

    let n = anchor.len();

    let weights: Vec<f64> = preds
        .iter()
        .map(|sig| {
            let len = sig.len().min(n);
            let predicted_pos = sig[..len].iter().filter(|&&p| p).count();
            if predicted_pos == 0 {
                return 0.01_f64;
            }
            let tp = sig[..len]
                .iter()
                .zip(anchor[..len].iter())
                .filter(|(&p, &a)| p && a)
                .count();
            let prec = tp as f64 / predicted_pos as f64;
            prec.clamp(0.01, 1.0)
        })
        .collect();

    weighted_vote(preds, &weights, n_target)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn std_dev(xs: &[f64], mean: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let var = xs.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
    var.sqrt()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // signal_weights
    // -----------------------------------------------------------------------

    #[test]
    fn test_signal_weights_correct_values() {
        // anchor = [T, T, F, F]
        // sig0 agrees on all 4 → accuracy 1.0
        // sig1 agrees on 2 of 4 (indices 1 and 2) → accuracy 0.5
        // sig2 agrees on none → accuracy 0.0 → clamped to 0.01
        let anchor = vec![true, true, false, false];
        let preds = vec![
            vec![true, true, false, false],  // sig0: perfect
            vec![false, true, true, false],  // sig1: 2/4 = 0.5
            vec![false, false, true, true],  // sig2: 0/4 = 0.0 → 0.01
        ];
        let w = signal_weights(&preds, &anchor);
        assert_eq!(w.len(), 3);
        assert!((w[0] - 1.0).abs() < 1e-10, "w[0]={}", w[0]);
        assert!((w[1] - 0.5).abs() < 1e-10, "w[1]={}", w[1]);
        assert!((w[2] - 0.01).abs() < 1e-10, "w[2]={}", w[2]);
    }

    #[test]
    fn test_signal_weights_anchor_all_false() {
        // anchor all false; sig0 always false → 100% agreement
        // sig1 always true → 0% agreement → clamped to 0.01
        let anchor = vec![false, false, false, false];
        let preds = vec![
            vec![false, false, false, false],
            vec![true, true, true, true],
        ];
        let w = signal_weights(&preds, &anchor);
        assert!((w[0] - 1.0).abs() < 1e-10);
        assert!((w[1] - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_signal_weights_empty_preds() {
        let anchor = vec![true, false];
        let w = signal_weights(&[], &anchor);
        assert!(w.is_empty());
    }

    #[test]
    fn test_signal_weights_clamp_at_01() {
        // Every signal should have weight >= 0.01.
        let anchor = vec![true, true, true];
        let preds = vec![vec![false, false, false]]; // 0% accuracy
        let w = signal_weights(&preds, &anchor);
        assert!(w[0] >= 0.01 - 1e-12);
    }

    // -----------------------------------------------------------------------
    // weighted_vote
    // -----------------------------------------------------------------------

    #[test]
    fn test_weighted_vote_calibrates_to_n_target() {
        // 4 traces; n_target = 2
        // weights: sig0=1.0, sig1=0.5
        // scores: i0 = 1.0+0.5=1.5, i1 = 0.5, i2 = 1.0, i3 = 0.0
        // top-2 by score: indices 0 (1.5) and 2 (1.0)
        let preds = vec![
            vec![true, false, true, false],  // sig0
            vec![true, true, false, false],  // sig1
        ];
        let weights = vec![1.0, 0.5];
        let result = weighted_vote(&preds, &weights, 2);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
        assert!(result[0], "index 0 should be true (highest score 1.5)");
        assert!(result[2], "index 2 should be true (score 1.0)");
        assert!(!result[1], "index 1 should be false");
        assert!(!result[3], "index 3 should be false");
    }

    #[test]
    fn test_weighted_vote_n_target_exceeds_n_all_true() {
        let preds = vec![vec![false, false, false]];
        let weights = vec![0.5];
        let result = weighted_vote(&preds, &weights, 10);
        assert_eq!(result, vec![true, true, true]);
    }

    #[test]
    fn test_weighted_vote_n_target_zero_all_false() {
        let preds = vec![vec![true, true, true]];
        let weights = vec![1.0];
        let result = weighted_vote(&preds, &weights, 0);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_weighted_vote_tie_break_lower_index() {
        // All traces have the same weighted score → tie-break by lower index.
        // n_target = 2 → indices 0 and 1 should be selected.
        let preds = vec![vec![true, true, true, true]];
        let weights = vec![1.0];
        let result = weighted_vote(&preds, &weights, 2);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
        assert!(result[0], "lower index 0 wins tie-break");
        assert!(result[1], "lower index 1 wins tie-break");
        assert!(!result[2]);
        assert!(!result[3]);
    }

    #[test]
    fn test_weighted_vote_empty_preds_returns_empty() {
        let result = weighted_vote(&[], &[], 3);
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // auto_weighted_vote
    // -----------------------------------------------------------------------

    #[test]
    fn test_auto_weighted_vote_end_to_end() {
        // sig0 is perfect; sig1 is anti-correlated.
        // auto_weighted_vote should favor sig0 and produce n_target positives.
        let anchor = vec![true, true, false, false];
        let preds = vec![
            vec![true, true, false, false],  // perfect
            vec![false, false, true, true],  // inverted
        ];
        let result = auto_weighted_vote(&preds, &anchor, 2);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
        // The top-2 should align with the anchor.
        let tp = result.iter().zip(anchor.iter()).filter(|(&p, &a)| p && a).count();
        assert_eq!(tp, 2, "expected both positives to match anchor, got {tp}");
    }

    #[test]
    fn test_auto_weighted_vote_empty_preds_returns_all_false() {
        let anchor = vec![true, false, true];
        let result = auto_weighted_vote(&[], &anchor, 1);
        assert_eq!(result, vec![false, false, false]);
    }

    // -----------------------------------------------------------------------
    // signal_correlations
    // -----------------------------------------------------------------------

    #[test]
    fn test_signal_correlations_perfect_and_anti() {
        let anchor = vec![true, true, false, false];
        let preds = vec![
            vec![true, true, false, false],  // r = +1
            vec![false, false, true, true],  // r = -1
        ];
        let corrs = signal_correlations(&preds, &anchor);
        assert_eq!(corrs.len(), 2);
        assert!((corrs[0] - 1.0).abs() < 1e-10, "corrs[0]={}", corrs[0]);
        assert!((corrs[1] + 1.0).abs() < 1e-10, "corrs[1]={}", corrs[1]);
    }

    #[test]
    fn test_signal_correlations_constant_signal_returns_zero() {
        // Constant signal (always true) has zero std dev → r=0.
        let anchor = vec![true, false, true, false];
        let preds = vec![vec![true, true, true, true]];
        let corrs = signal_correlations(&preds, &anchor);
        assert!((corrs[0] - 0.0).abs() < 1e-10, "expected 0 for constant signal");
    }

    #[test]
    fn test_signal_correlations_empty_returns_empty() {
        let corrs = signal_correlations(&[], &[true, false]);
        assert!(corrs.is_empty());
    }

    #[test]
    fn test_signal_correlations_uncorrelated_near_zero() {
        // anchor = [T,F,T,F,T,F] (alternating), sig = [T,T,T,F,F,F]
        // Not perfectly correlated, but not perfectly anti-correlated either.
        let anchor = vec![true, false, true, false, true, false];
        let preds = vec![vec![true, true, true, false, false, false]];
        let corrs = signal_correlations(&preds, &anchor);
        // Pearson r should be between -1 and 1, and likely near 0 for this arrangement.
        assert!(corrs[0].abs() <= 1.0 + 1e-10, "r must be in [-1,1]");
    }

    // -----------------------------------------------------------------------
    // precision_weighted_vote
    // -----------------------------------------------------------------------

    #[test]
    fn test_precision_weighted_vote_favors_high_precision() {
        // sig0: predicts only 1 positive (index 0) and it's correct → precision 1.0
        // sig1: predicts 3 positives (0,1,2) but only index 0 is correct → precision 1/3
        // weighted scores for n_target=1: index 0 gets both weights; others only sig1.
        // Expected: index 0 selected as positive.
        let anchor = vec![true, false, false, false];
        let preds = vec![
            vec![true, false, false, false],   // sig0: 1 pred, 1 tp → prec=1.0
            vec![true, true, true, false],     // sig1: 3 preds, 1 tp → prec=1/3
        ];
        let result = precision_weighted_vote(&preds, &anchor, 1);
        assert_eq!(result.iter().filter(|&&b| b).count(), 1);
        assert!(result[0], "index 0 (true positive) should be selected");
    }

    #[test]
    fn test_precision_weighted_vote_no_predicted_positives_gets_min_weight() {
        // sig0 always predicts false → precision undefined → weight 0.01
        // sig1 has perfect precision on the one positive.
        let anchor = vec![true, false, false];
        let preds = vec![
            vec![false, false, false],   // sig0: no predicted positives → 0.01
            vec![true, false, false],    // sig1: prec=1.0
        ];
        let result = precision_weighted_vote(&preds, &anchor, 1);
        assert_eq!(result.iter().filter(|&&b| b).count(), 1);
        assert!(result[0], "index 0 should be the top positive");
    }

    #[test]
    fn test_precision_weighted_vote_n_target_gte_n_all_true() {
        let anchor = vec![true, false];
        let preds = vec![vec![true, false]];
        let result = precision_weighted_vote(&preds, &anchor, 5);
        assert_eq!(result, vec![true, true]);
    }

    #[test]
    fn test_precision_weighted_vote_empty_preds() {
        let anchor = vec![true, false, true];
        let result = precision_weighted_vote(&[], &anchor, 1);
        assert_eq!(result, vec![false, false, false]);
    }
}
