//! Combinatorial ensemble search for PDC 2025.
//!
//! Given predictions from up to ~17 classifiers for N traces, exhaustively
//! (or greedily) searches for the subset of classifiers whose majority vote
//! best agrees with high-precision in-language BFS pseudo-labels while
//! selecting exactly `n_target` positives.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Try all 2^k non-empty subsets of classifier predictions via majority vote.
/// Returns the calibrated prediction vector with exactly `n_target` positives.
///
/// # Arguments
/// * `all_preds` — k classifier prediction vectors, each of length ≥ n
///   (truncated to the common minimum length internally)
/// * `anchor`    — in-language BFS pseudo-labels (high-precision truth signal)
/// * `n_target`  — desired number of positives (e.g. 500 for PDC 2025)
///
/// # Scoring
/// For each subset, majority vote gives raw predictions, then:
/// ```text
/// score = recall_on_anchor + precision_penalty
/// recall_on_anchor = |{i: anchor[i] && vote[i]}| / max(|anchor=true|, 1)
/// precision_penalty = -0.1 * |count_positives(vote) - n_target| / n_target
/// ```
///
/// # Calibration
/// After finding the best subset, per-trace vote fractions are computed over
/// that subset; traces are sorted descending by fraction and the top `n_target`
/// are set true.
///
/// # Fallback
/// For k > 20 the exhaustive 2^k search is skipped in favour of
/// `greedy_ensemble`, which runs in O(k² · n) time.
pub fn combinatorial_ensemble(
    all_preds: &[Vec<bool>],
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    let k = all_preds.len();

    if k == 0 {
        return vec![false; anchor.len()];
    }

    // Common length across all classifier vectors (truncate to minimum).
    let n = all_preds
        .iter()
        .map(|v| v.len())
        .min()
        .unwrap_or(0)
        .min(anchor.len());

    if n == 0 {
        return vec![false; anchor.len()];
    }

    // Exponential cost: 2^k iterations. Cap at k=12 (4096) — above that use greedy.
    // Empirically, optimal subsets are 3-5 classifiers; greedy finds them well at k>12.
    if k > 12 {
        return greedy_ensemble(all_preds, anchor, n_target);
    }

    let mut best_score = f64::NEG_INFINITY;
    let mut best_mask: u64 = 1; // default: first classifier alone

    // Iterate every non-empty subset encoded as a bitmask.
    for mask in 1u64..(1u64 << k) {
        let subset: Vec<usize> = (0..k).filter(|&i| mask & (1u64 << i) != 0).collect();
        let vote = majority_vote_n(all_preds, &subset, n);
        let s = score(&vote, &anchor[..n], n_target);
        if s > best_score {
            best_score = s;
            best_mask = mask;
        }
    }

    let best_subset: Vec<usize> = (0..k).filter(|&i| best_mask & (1u64 << i) != 0).collect();
    let raw_vote = majority_vote_n(all_preds, &best_subset, n);

    // Per-trace vote fractions for the best subset only (used for calibration).
    let fracs = vote_fractions_subset(all_preds, &best_subset, n);
    calibrate_to_target(&raw_vote, &fracs, n_target)
}

/// Greedy forward selection: start from an empty set, repeatedly add the
/// classifier that most improves the score.  Stops when no single addition
/// yields improvement.
///
/// After finding the best subset, calibrates to exactly `n_target` positives.
pub fn greedy_ensemble(all_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> Vec<bool> {
    let k = all_preds.len();

    if k == 0 {
        return vec![false; anchor.len()];
    }

    let n = all_preds
        .iter()
        .map(|v| v.len())
        .min()
        .unwrap_or(0)
        .min(anchor.len());

    if n == 0 {
        return vec![false; anchor.len()];
    }

    let mut selected: Vec<usize> = Vec::with_capacity(k);
    let mut remaining: Vec<usize> = (0..k).collect();
    let mut best_score = f64::NEG_INFINITY;

    loop {
        if remaining.is_empty() {
            break;
        }

        let mut round_best_score = best_score;
        let mut round_best_ri: Option<usize> = None; // index into `remaining`

        for (ri, &clf) in remaining.iter().enumerate() {
            // Build candidate subset without cloning `selected` entirely every time.
            let candidate_len = selected.len() + 1;
            let vote = {
                // Inline subset iteration to avoid a heap allocation per round.
                let mut v = vec![false; n];
                for i in 0..n {
                    let trues: usize = selected
                        .iter()
                        .chain(std::iter::once(&clf))
                        .filter(|&&c| all_preds[c][i])
                        .count();
                    v[i] = trues * 2 > candidate_len;
                }
                v
            };
            let s = score(&vote, &anchor[..n], n_target);

            if s > round_best_score {
                round_best_score = s;
                round_best_ri = Some(ri);
            }
        }

        match round_best_ri {
            None => break, // no improvement — stop
            Some(ri) => {
                best_score = round_best_score;
                selected.push(remaining.remove(ri));
            }
        }
    }

    // Edge case: nothing ever improved (single-classifier baseline still negative).
    if selected.is_empty() {
        selected.push(0);
    }

    let raw_vote = majority_vote_n(all_preds, &selected, n);
    let fracs = vote_fractions_subset(all_preds, &selected, n);
    calibrate_to_target(&raw_vote, &fracs, n_target)
}

/// Majority vote of a specific subset of classifiers (given by indices).
///
/// Returns raw bool predictions of length equal to the minimum length across
/// the selected classifier vectors.  A trace is predicted positive if strictly
/// more than half of the subset votes true; ties resolve to `false`.
pub fn majority_vote(all_preds: &[Vec<bool>], subset: &[usize]) -> Vec<bool> {
    if subset.is_empty() || all_preds.is_empty() {
        return Vec::new();
    }
    let n = subset
        .iter()
        .map(|&i| all_preds[i].len())
        .min()
        .unwrap_or(0);
    majority_vote_n(all_preds, subset, n)
}

/// Score a prediction vector against the anchor and `n_target`.
///
/// ```text
/// score = recall_on_anchor + precision_penalty
/// recall_on_anchor = |{i: anchor[i] && vote[i]}| / max(|anchor=true|, 1)
/// precision_penalty = -0.1 * |count_positives(vote) - n_target| / n_target
/// ```
pub fn score(preds: &[bool], anchor: &[bool], n_target: usize) -> f64 {
    let n = preds.len().min(anchor.len());
    if n == 0 {
        return 0.0;
    }

    let anchor_pos = anchor[..n].iter().filter(|&&a| a).count();
    let tp = preds[..n]
        .iter()
        .zip(anchor[..n].iter())
        .filter(|(&p, &a)| p && a)
        .count();

    let recall = tp as f64 / anchor_pos.max(1) as f64;

    let count_pos = preds[..n].iter().filter(|&&p| p).count();
    let n_target_f = n_target.max(1) as f64;
    let precision_penalty = -0.1 * (count_pos as f64 - n_target_f).abs() / n_target_f;

    recall + precision_penalty
}

/// For each trace, compute the fraction of classifiers that vote true across
/// ALL provided classifiers.
///
/// Length of result equals the minimum length across all classifier vectors.
pub fn vote_fractions(all_preds: &[Vec<bool>]) -> Vec<f64> {
    if all_preds.is_empty() {
        return Vec::new();
    }
    let n = all_preds.iter().map(|v| v.len()).min().unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    let k = all_preds.len() as f64;
    (0..n)
        .map(|i| {
            let trues = all_preds.iter().filter(|v| v[i]).count();
            trues as f64 / k
        })
        .collect()
}

/// Calibrate raw predictions to exactly `n_target` positives by ranking
/// per-trace vote fractions descending.
///
/// Traces ranked highest get `true` until `n_target` are filled.  Ties at the
/// boundary are broken by original index (stable, deterministic).
pub fn calibrate_to_target(preds: &[bool], vote_fracs: &[f64], n_target: usize) -> Vec<bool> {
    let n = preds.len().min(vote_fracs.len());

    // Build index sorted by frac descending (stable on original order for ties).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        vote_fracs[b]
            .partial_cmp(&vote_fracs[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Majority vote over a subset of classifiers, truncated to the first `n`
/// traces.  Tie-breaks to `false` (strictly > half must vote true).
fn majority_vote_n(all_preds: &[Vec<bool>], subset: &[usize], n: usize) -> Vec<bool> {
    if subset.is_empty() || n == 0 {
        return vec![false; n];
    }
    let half = subset.len();
    (0..n)
        .map(|i| {
            let trues = subset.iter().filter(|&&clf| all_preds[clf][i]).count();
            trues * 2 > half // strictly more than half → true; tie → false
        })
        .collect()
}

/// Vote fractions restricted to a given subset of classifiers, first `n` traces.
fn vote_fractions_subset(all_preds: &[Vec<bool>], subset: &[usize], n: usize) -> Vec<f64> {
    if subset.is_empty() || n == 0 {
        return vec![0.0; n];
    }
    let k = subset.len() as f64;
    (0..n)
        .map(|i| {
            let trues = subset.iter().filter(|&&clf| all_preds[clf][i]).count();
            trues as f64 / k
        })
        .collect()
}

/// Full combinatorial search over BOTH boolean predictions AND continuous score signals.
///
/// `bool_preds`: existing binary classifier outputs (Vec<Vec<bool>>)
/// `score_signals`: continuous scored signals (e.g. fitness, neg_edit_dist, in_lang as f64);
///   higher values = more likely positive for all signals.
/// `anchor`: in-language BFS results used for scoring.
/// `n_target`: 500 for PDC 2025.
///
/// Algorithm:
/// 1. Convert each score_signal to bool by top-`n_target` threshold → add to bool pool.
/// 2. Run `combinatorial_ensemble` on the combined pool (bool_preds + converted scores).
/// 3. Return result.
pub fn full_combinatorial(
    bool_preds: &[Vec<bool>],
    score_signals: &[Vec<f64>],
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    // Convert each continuous signal to a bool vector via top-n_target threshold.
    let converted: Vec<Vec<bool>> = score_signals
        .iter()
        .map(|sig| score_signal_to_bool(sig, n_target))
        .collect();

    // Build the combined pool: original bool preds + converted score signals.
    let mut pool: Vec<Vec<bool>> = bool_preds.to_vec();
    pool.extend(converted);

    if pool.is_empty() {
        return vec![false; anchor.len()];
    }

    combinatorial_ensemble(&pool, anchor, n_target)
}

/// Try all pairs (one bool_pred, one score_signal):
/// for each pair take traces where `bool_pred=true`, among those take top-k by
/// `score_signal`, plus fill remaining slots from `bool_pred=false` sorted by
/// `score_signal` descending.
/// Return the pair that maximises anchor agreement.
///
/// If either input slice is empty, returns all-false of length `anchor.len()`.
pub fn best_bool_score_pair(
    bool_preds: &[Vec<bool>],
    score_signals: &[Vec<f64>],
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    if bool_preds.is_empty() || score_signals.is_empty() {
        return vec![false; anchor.len()];
    }

    let n = bool_preds
        .iter()
        .map(|v| v.len())
        .chain(score_signals.iter().map(|v| v.len()))
        .min()
        .unwrap_or(0)
        .min(anchor.len());

    if n == 0 {
        return vec![false; anchor.len()];
    }

    let mut best_score = f64::NEG_INFINITY;
    let mut best_result: Option<Vec<bool>> = None;

    for bp in bool_preds.iter() {
        for sig in score_signals.iter() {
            let candidate = pair_prediction(bp, sig, n, n_target);
            let s = score(&candidate, &anchor[..n], n_target);
            if s > best_score {
                best_score = s;
                best_result = Some(candidate);
            }
        }
    }

    best_result.unwrap_or_else(|| vec![false; n])
}

// ---------------------------------------------------------------------------
// Internal helpers (continued)
// ---------------------------------------------------------------------------

/// Convert a continuous score vector to bools by marking the top `n_target`
/// indices as `true` (sorted descending, ties broken by original index).
fn score_signal_to_bool(sig: &[f64], n_target: usize) -> Vec<bool> {
    let n = sig.len();
    if n == 0 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        sig[b]
            .partial_cmp(&sig[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

/// Build a prediction vector for a (bool_pred, score_signal) pair:
/// - Take indices where bool_pred=true, sorted by score_signal descending.
///   Keep the top min(n_target, pos_count) as definite positives.
/// - Fill remaining slots from bool_pred=false indices, sorted by score_signal descending.
///
/// Result length is `n`.
fn pair_prediction(bp: &[bool], sig: &[f64], n: usize, n_target: usize) -> Vec<bool> {
    let mut pos_indices: Vec<usize> = (0..n).filter(|&i| bp[i]).collect();
    let mut neg_indices: Vec<usize> = (0..n).filter(|&i| !bp[i]).collect();

    // Sort both groups by score descending.
    pos_indices.sort_by(|&a, &b| {
        sig[b]
            .partial_cmp(&sig[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    neg_indices.sort_by(|&a, &b| {
        sig[b]
            .partial_cmp(&sig[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let take_from_pos = pos_indices.len().min(n_target);
    let remaining = n_target.saturating_sub(take_from_pos);

    let mut result = vec![false; n];
    for &idx in pos_indices.iter().take(take_from_pos) {
        result[idx] = true;
    }
    for &idx in neg_indices.iter().take(remaining) {
        result[idx] = true;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // majority_vote
    // ------------------------------------------------------------------

    #[test]
    fn test_majority_vote_basic() {
        // clf0: T F T F
        // clf1: T T T F
        // clf2: F F T F
        // majority (need > 1.5 out of 3): T F T F
        let preds = vec![
            vec![true, false, true, false],
            vec![true, true, true, false],
            vec![false, false, true, false],
        ];
        let result = majority_vote(&preds, &[0, 1, 2]);
        assert_eq!(result, vec![true, false, true, false]);
    }

    #[test]
    fn test_majority_vote_tie_breaks_false() {
        // 2 classifiers: [T, F] vs [F, T] → ties → all false
        let preds = vec![vec![true, false], vec![false, true]];
        let result = majority_vote(&preds, &[0, 1]);
        assert_eq!(result, vec![false, false]);
    }

    #[test]
    fn test_majority_vote_single_classifier() {
        let preds = vec![vec![true, false, true]];
        let result = majority_vote(&preds, &[0]);
        assert_eq!(result, vec![true, false, true]);
    }

    #[test]
    fn test_majority_vote_empty_subset_returns_empty() {
        let preds = vec![vec![true, false]];
        let result = majority_vote(&preds, &[]);
        assert!(result.is_empty());
    }

    // ------------------------------------------------------------------
    // score
    // ------------------------------------------------------------------

    #[test]
    fn test_score_perfect_recall_exact_target() {
        // anchor = [T,T,F,F], preds = [T,T,F,F], n_target = 2
        // recall = 2/2 = 1.0, penalty = -0.1 * 0 / 2 = 0.0 → 1.0
        let anchor = vec![true, true, false, false];
        let preds = vec![true, true, false, false];
        let s = score(&preds, &anchor, 2);
        assert!((s - 1.0).abs() < 1e-10, "score={s}");
    }

    #[test]
    fn test_score_zero_recall() {
        // anchor = [T,T,F,F], preds = [F,F,T,T] → recall=0, count_pos=2=n_target → penalty=0
        let anchor = vec![true, true, false, false];
        let preds = vec![false, false, true, true];
        let s = score(&preds, &anchor, 2);
        assert!((s - 0.0).abs() < 1e-10, "score={s}");
    }

    #[test]
    fn test_score_penalty_for_wrong_count() {
        // anchor = [T,F,F,F], preds = [T,T,T,T], n_target=1
        // recall = 1/1 = 1.0, penalty = -0.1 * |4-1|/1 = -0.3 → 0.7
        let anchor = vec![true, false, false, false];
        let preds = vec![true, true, true, true];
        let s = score(&preds, &anchor, 1);
        assert!((s - 0.7).abs() < 1e-10, "score={s}");
    }

    // ------------------------------------------------------------------
    // combinatorial_ensemble (3 classifiers — exhaustive 2^3 = 8 subsets)
    // ------------------------------------------------------------------

    #[test]
    fn test_combinatorial_ensemble_3_classifiers() {
        // clf0: T T T F F F  ← best aligned with anchor
        // clf1: T F F F F F
        // clf2: F F F T T T  ← anti-correlated
        // anchor = [T,T,T,F,F,F], n_target=3
        let preds = vec![
            vec![true, true, true, false, false, false],
            vec![true, false, false, false, false, false],
            vec![false, false, false, true, true, true],
        ];
        let anchor = vec![true, true, true, false, false, false];
        let result = combinatorial_ensemble(&preds, &anchor, 3);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(
            pos_count, 3,
            "expected exactly 3 positives, got {pos_count}"
        );

        let tp = result
            .iter()
            .zip(anchor.iter())
            .filter(|(&p, &a)| p && a)
            .count();
        assert_eq!(tp, 3, "expected all 3 anchor positives, got {tp}");
    }

    #[test]
    fn test_combinatorial_ensemble_empty_input() {
        let result = combinatorial_ensemble(&[], &[true, false, true], 1);
        assert_eq!(result, vec![false, false, false]);
    }

    // ------------------------------------------------------------------
    // greedy_ensemble
    // ------------------------------------------------------------------

    #[test]
    fn test_greedy_ensemble_selects_best_classifier() {
        // clf0 is perfect; clf1 introduces noise.
        let preds = vec![
            vec![true, true, false, false],
            vec![false, true, true, false],
        ];
        let anchor = vec![true, true, false, false];
        let result = greedy_ensemble(&preds, &anchor, 2);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(pos_count, 2, "expected 2 positives, got {pos_count}");

        let tp = result
            .iter()
            .zip(anchor.iter())
            .filter(|(&p, &a)| p && a)
            .count();
        assert_eq!(tp, 2, "expected both anchor positives recovered, got {tp}");
    }

    #[test]
    fn test_greedy_ensemble_empty_input() {
        let result = greedy_ensemble(&[], &[true, false], 1);
        assert_eq!(result, vec![false, false]);
    }

    // ------------------------------------------------------------------
    // calibrate_to_target
    // ------------------------------------------------------------------

    #[test]
    fn test_calibrate_to_target_exact() {
        // fracs: [0.8, 0.6, 0.4, 0.2] — top-2 are indices 0 and 1
        let preds = vec![true, false, true, false];
        let fracs = vec![0.8, 0.6, 0.4, 0.2];
        let result = calibrate_to_target(&preds, &fracs, 2);
        assert_eq!(result, vec![true, true, false, false]);
    }

    #[test]
    fn test_calibrate_to_target_zero() {
        let preds = vec![true, true, true];
        let fracs = vec![0.9, 0.7, 0.5];
        let result = calibrate_to_target(&preds, &fracs, 0);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_calibrate_to_target_all() {
        let preds = vec![false, false, false];
        let fracs = vec![0.1, 0.5, 0.3];
        let result = calibrate_to_target(&preds, &fracs, 3);
        assert_eq!(result, vec![true, true, true]);
    }

    // ------------------------------------------------------------------
    // vote_fractions
    // ------------------------------------------------------------------

    #[test]
    fn test_vote_fractions_basic() {
        let preds = vec![vec![true, false, true], vec![true, true, false]];
        let fracs = vote_fractions(&preds);
        assert_eq!(fracs.len(), 3);
        assert!((fracs[0] - 1.0).abs() < 1e-10, "fracs[0]={}", fracs[0]);
        assert!((fracs[1] - 0.5).abs() < 1e-10, "fracs[1]={}", fracs[1]);
        assert!((fracs[2] - 0.5).abs() < 1e-10, "fracs[2]={}", fracs[2]);
    }

    #[test]
    fn test_vote_fractions_empty() {
        assert!(vote_fractions(&[]).is_empty());
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_combinatorial_ensemble_differing_lengths() {
        // Classifier vectors of different lengths — must truncate to minimum.
        let preds = vec![
            vec![true, true, true, true, true],
            vec![true, true, false], // shorter
        ];
        let anchor = vec![true, true, false, false, false];
        // n = min(5, 3, 5) = 3
        let result = combinatorial_ensemble(&preds, &anchor, 2);
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }

    #[test]
    fn test_score_empty_preds() {
        let s = score(&[], &[true, false], 1);
        assert!((s - 0.0).abs() < 1e-10);
    }

    // ------------------------------------------------------------------
    // full_combinatorial
    // ------------------------------------------------------------------

    #[test]
    fn test_full_combinatorial_uses_score_signals() {
        // bool_preds: one classifier that is all-false (useless).
        // score_signals: one signal that ranks traces 0-2 highest.
        // anchor: first 3 are true.
        // n_target = 3; the converted signal should push correct traces to top.
        let bool_preds = vec![vec![false; 6]];
        let score_signals = vec![vec![6.0, 5.0, 4.0, 1.0, 0.5, 0.0]];
        let anchor = vec![true, true, true, false, false, false];

        let result = full_combinatorial(&bool_preds, &score_signals, &anchor, 3);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(
            pos_count, 3,
            "expected exactly 3 positives, got {pos_count}"
        );

        let tp = result
            .iter()
            .zip(anchor.iter())
            .filter(|(&p, &a)| p && a)
            .count();
        assert_eq!(
            tp, 3,
            "all 3 anchor positives should be recovered, got {tp}"
        );
    }

    #[test]
    fn test_full_combinatorial_empty_both() {
        let result = full_combinatorial(&[], &[], &[true, false, true], 1);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_full_combinatorial_empty_score_signals() {
        // Without score signals the function should still work using bool_preds only.
        let bool_preds = vec![vec![true, false, true, false]];
        let anchor = vec![true, false, true, false];

        let result = full_combinatorial(&bool_preds, &[], &anchor, 2);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(pos_count, 2, "expected 2 positives, got {pos_count}");
    }

    // ------------------------------------------------------------------
    // best_bool_score_pair
    // ------------------------------------------------------------------

    #[test]
    fn test_best_bool_score_pair_selects_correct_pair() {
        // 4 traces, n_target = 2.
        // bool_preds[0]: [T, T, F, F]  ← aligned with anchor
        // bool_preds[1]: [F, F, T, T]  ← anti-aligned
        // score_signals[0]: [0.9, 0.8, 0.3, 0.1]  ← high for anchor traces
        // anchor: [T, T, F, F]
        //
        // Best pair should be (bool_preds[0], score_signals[0]):
        //   pos_by_bool = [0,1], top-2 = [0,1]; result = [T,T,F,F] → tp=2
        let bool_preds = vec![
            vec![true, true, false, false],
            vec![false, false, true, true],
        ];
        let score_signals = vec![vec![0.9, 0.8, 0.3, 0.1]];
        let anchor = vec![true, true, false, false];

        let result = best_bool_score_pair(&bool_preds, &score_signals, &anchor, 2);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(pos_count, 2, "expected 2 positives, got {pos_count}");

        let tp = result
            .iter()
            .zip(anchor.iter())
            .filter(|(&p, &a)| p && a)
            .count();
        assert_eq!(tp, 2, "both anchor positives should be recovered, got {tp}");
    }

    #[test]
    fn test_best_bool_score_pair_fills_from_negatives() {
        // 6 traces, n_target = 4.
        // bool_preds[0] = [T, T, F, F, F, F]  (only 2 positives, need 4)
        // score_signals[0] = [0.1, 0.2, 0.9, 0.8, 0.7, 0.3]
        // Fill 2 remaining from neg group sorted by signal: idx 2 (0.9), idx 3 (0.8)
        // anchor = [T, T, T, T, F, F]
        let bool_preds = vec![vec![true, true, false, false, false, false]];
        let score_signals = vec![vec![0.1, 0.2, 0.9, 0.8, 0.7, 0.3]];
        let anchor = vec![true, true, true, true, false, false];

        let result = best_bool_score_pair(&bool_preds, &score_signals, &anchor, 4);

        let pos_count = result.iter().filter(|&&b| b).count();
        assert_eq!(pos_count, 4, "expected 4 positives, got {pos_count}");

        // Indices 0,1 (from bool=true) and 2,3 (top negatives by signal) should be true.
        assert!(result[0], "index 0 should be positive");
        assert!(result[1], "index 1 should be positive");
        assert!(
            result[2],
            "index 2 should be positive (top negative by signal)"
        );
        assert!(
            result[3],
            "index 3 should be positive (2nd negative by signal)"
        );
        assert!(!result[4], "index 4 should be negative");
        assert!(!result[5], "index 5 should be negative");
    }

    #[test]
    fn test_best_bool_score_pair_empty_bool_preds() {
        let score_signals = vec![vec![1.0, 0.5, 0.0]];
        let anchor = vec![true, false, true];
        let result = best_bool_score_pair(&[], &score_signals, &anchor, 1);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_best_bool_score_pair_empty_score_signals() {
        let bool_preds = vec![vec![true, false, true]];
        let anchor = vec![true, false, true];
        let result = best_bool_score_pair(&bool_preds, &[], &anchor, 1);
        assert_eq!(result, vec![false, false, false]);
    }
}
