/// Rank fusion utilities: Borda count and Reciprocal Rank Fusion (RRF).
///
/// Both functions operate on a matrix of signal scores:
///   `scores[k][i]` = signal k's score for trace i.
/// `higher_is_better[k]` indicates the polarity of signal k.
/// `n_target` is the number of top traces to select (returned as `true`).
/// Rank a score vector: returns `rank[i]` = the zero-based rank of trace `i`
/// when all traces are sorted best-first.
///
/// Ties are broken by lower original index (first-seen wins).
fn rank_signal(scores: &[f64], higher_is_better: bool) -> Vec<usize> {
    let n = scores.len();
    // Build (score, original_index) pairs and sort best-first.
    let mut order: Vec<usize> = (0..n).collect();
    if higher_is_better {
        order.sort_by(|&a, &b| {
            scores[b]
                .partial_cmp(&scores[a])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b)) // tie: lower index first
        });
    } else {
        order.sort_by(|&a, &b| {
            scores[a]
                .partial_cmp(&scores[b])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
    }
    // Invert: rank[i] = position of trace i in sorted order.
    let mut rank = vec![0usize; n];
    for (pos, &idx) in order.iter().enumerate() {
        rank[idx] = pos;
    }
    rank
}

/// Borda count fusion over multiple scored signals.
///
/// For each signal `k`, rank every trace (rank 0 = best).
/// Borda score for trace `i` = Σ_k (n − rank_k[i]).
/// Returns a `Vec<bool>` of length `n` where the top `n_target` traces are `true`.
///
/// Edge cases:
/// - `scores` empty or `n_target == 0` → all `false`.
/// - `n_target >= n` → all `true`.
/// - All scores equal within a signal → all traces share rank 0 for that signal.
pub fn borda_count(scores: &[Vec<f64>], higher_is_better: &[bool], n_target: usize) -> Vec<bool> {
    let k = scores.len();
    if k == 0 || n_target == 0 {
        // Determine n if possible.
        let n = scores.first().map(|v| v.len()).unwrap_or(0);
        return vec![false; n];
    }
    let n = scores[0].len();
    if n == 0 {
        return vec![];
    }
    if n_target >= n {
        return vec![true; n];
    }

    // Accumulate Borda scores.
    let mut borda = vec![0usize; n];
    for (k_idx, signal) in scores.iter().enumerate() {
        let hib = higher_is_better.get(k_idx).copied().unwrap_or(true);
        let ranks = rank_signal(signal, hib);
        for i in 0..n {
            borda[i] += n - ranks[i];
        }
    }

    // Find top n_target by Borda score (ties: lower index first).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| borda[b].cmp(&borda[a]).then(a.cmp(&b)));

    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

/// Reciprocal Rank Fusion.
///
/// RRF score for trace `i` = Σ_k  1.0 / (k_const + rank_k[i] + 1)
/// where `k_const = 60.0` (standard RRF parameter).
///
/// Returns a `Vec<bool>` of length `n` where the top `n_target` traces are `true`.
///
/// Same edge cases as [`borda_count`].
pub fn reciprocal_rank_fusion(
    scores: &[Vec<f64>],
    higher_is_better: &[bool],
    n_target: usize,
) -> Vec<bool> {
    const K_CONST: f64 = 60.0;

    let k = scores.len();
    if k == 0 || n_target == 0 {
        let n = scores.first().map(|v| v.len()).unwrap_or(0);
        return vec![false; n];
    }
    let n = scores[0].len();
    if n == 0 {
        return vec![];
    }
    if n_target >= n {
        return vec![true; n];
    }

    let mut rrf = vec![0.0f64; n];
    for (k_idx, signal) in scores.iter().enumerate() {
        let hib = higher_is_better.get(k_idx).copied().unwrap_or(true);
        let ranks = rank_signal(signal, hib);
        for i in 0..n {
            rrf[i] += 1.0 / (K_CONST + ranks[i] as f64 + 1.0);
        }
    }

    // Top n_target by RRF score (ties: lower index first).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        rrf[b]
            .partial_cmp(&rrf[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });

    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

/// Convert boolean predictions to `f64` scores (`true` → 1.0, `false` → 0.0).
pub fn bool_to_score(preds: &[bool]) -> Vec<f64> {
    preds.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect()
}

/// Convert edit distances (lower = better) to `f64` scores by negation.
/// The returned scores are `-(dist as f64)`, so higher is better.
pub fn edit_dist_to_score(dists: &[usize]) -> Vec<f64> {
    dists.iter().map(|&d| -(d as f64)).collect()
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── rank_signal ────────────────────────────────────────────────────────────

    #[test]
    fn test_rank_signal_higher_is_better() {
        // scores: [3.0, 1.0, 2.0]
        // sorted descending: indices [0, 2, 1] → ranks: 0→0, 1→2, 2→1
        let scores = vec![3.0, 1.0, 2.0];
        let ranks = rank_signal(&scores, true);
        assert_eq!(ranks, vec![0, 2, 1]);
    }

    #[test]
    fn test_rank_signal_lower_is_better() {
        // scores: [3.0, 1.0, 2.0]
        // sorted ascending: indices [1, 2, 0] → ranks: 0→2, 1→0, 2→1
        let scores = vec![3.0, 1.0, 2.0];
        let ranks = rank_signal(&scores, false);
        assert_eq!(ranks, vec![2, 0, 1]);
    }

    #[test]
    fn test_rank_signal_tie_breaking() {
        // All equal: ties broken by lower index first.
        // sorted order: [0, 1, 2] → each gets its position
        let scores = vec![5.0, 5.0, 5.0];
        let ranks = rank_signal(&scores, true);
        assert_eq!(ranks, vec![0, 1, 2]);
    }

    // ── bool_to_score ──────────────────────────────────────────────────────────

    #[test]
    fn test_bool_to_score() {
        let preds = vec![true, false, true, false];
        let scores = bool_to_score(&preds);
        assert_eq!(scores, vec![1.0, 0.0, 1.0, 0.0]);
    }

    // ── edit_dist_to_score ─────────────────────────────────────────────────────

    #[test]
    fn test_edit_dist_to_score() {
        let dists = vec![0usize, 3, 1];
        let scores = edit_dist_to_score(&dists);
        assert_eq!(scores, vec![0.0, -3.0, -1.0]);
    }

    // ── borda_count ────────────────────────────────────────────────────────────

    #[test]
    fn test_borda_count_two_signals() {
        // 4 traces, 2 signals (both higher_is_better).
        // Signal 0: [4.0, 3.0, 2.0, 1.0] → ranks [0,1,2,3]
        // Signal 1: [1.0, 4.0, 3.0, 2.0] → ranks [3,0,1,2]
        // n=4, Borda: i0 = (4-0)+(4-3)=5, i1=(4-1)+(4-0)=7, i2=(4-2)+(4-1)=5, i3=(4-3)+(4-2)=3
        // Sort desc by Borda (ties lower index first): i1(7), i0(5), i2(5), i3(3)
        // Top 2 → {1, 0} → result [true, true, false, false]
        let scores = vec![vec![4.0, 3.0, 2.0, 1.0], vec![1.0, 4.0, 3.0, 2.0]];
        let hib = vec![true, true];
        let result = borda_count(&scores, &hib, 2);
        assert_eq!(result, vec![true, true, false, false]);
    }

    #[test]
    fn test_borda_count_edge_empty() {
        // Empty signals → all false (n=0)
        let result: Vec<bool> = borda_count(&[], &[], 2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_borda_count_edge_n_target_zero() {
        let scores = vec![vec![1.0, 2.0, 3.0]];
        let result = borda_count(&scores, &[true], 0);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_borda_count_edge_n_target_ge_n() {
        // n_target >= n → all true
        let scores = vec![vec![1.0, 2.0, 3.0]];
        let result = borda_count(&scores, &[true], 10);
        assert_eq!(result, vec![true, true, true]);
    }

    #[test]
    fn test_borda_count_all_equal() {
        // All scores equal → all same Borda → top 2 of 4 are indices 0,1 (tie-break by index).
        let scores = vec![vec![5.0, 5.0, 5.0, 5.0]];
        let result = borda_count(&scores, &[true], 2);
        assert_eq!(result, vec![true, true, false, false]);
    }

    // ── reciprocal_rank_fusion ─────────────────────────────────────────────────

    #[test]
    fn test_rrf_two_signals() {
        // Same setup as borda test.
        // Signal 0 ranks: [0,1,2,3], Signal 1 ranks: [3,0,1,2]
        // RRF(i0) = 1/(61)+1/(64) ≈ 0.01639+0.01563 = 0.03202
        // RRF(i1) = 1/(62)+1/(61) ≈ 0.01613+0.01639 = 0.03252
        // RRF(i2) = 1/(63)+1/(62) ≈ 0.01587+0.01613 = 0.03200
        // RRF(i3) = 1/(64)+1/(63) ≈ 0.01563+0.01587 = 0.03150
        // Sorted: i1 > i0 > i2 > i3
        // Top 2 → {1, 0}
        let scores = vec![vec![4.0, 3.0, 2.0, 1.0], vec![1.0, 4.0, 3.0, 2.0]];
        let hib = vec![true, true];
        let result = reciprocal_rank_fusion(&scores, &hib, 2);
        assert_eq!(result, vec![true, true, false, false]);
    }

    #[test]
    fn test_rrf_edge_n_target_zero() {
        let scores = vec![vec![1.0, 2.0, 3.0]];
        let result = reciprocal_rank_fusion(&scores, &[true], 0);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_rrf_edge_n_target_ge_n() {
        let scores = vec![vec![1.0, 2.0]];
        let result = reciprocal_rank_fusion(&scores, &[true], 5);
        assert_eq!(result, vec![true, true]);
    }

    #[test]
    fn test_rrf_mixed_polarity() {
        // 3 traces: signal 0 higher_is_better=true, signal 1 lower_is_better.
        // Signal 0: [10.0, 5.0, 1.0]  → ranks [0,1,2]
        // Signal 1: [1.0, 3.0, 2.0]   lower→ ranks [0,2,1]  (1.0 is best, lowest)
        // RRF(i0) = 1/61 + 1/61 ≈ 0.03279
        // RRF(i1) = 1/62 + 1/63 ≈ 0.03198
        // RRF(i2) = 1/63 + 1/62 ≈ 0.03198  (tie; i1 wins by lower index)
        // Top 1 → {0}
        let scores = vec![vec![10.0, 5.0, 1.0], vec![1.0, 3.0, 2.0]];
        let hib = vec![true, false];
        let result = reciprocal_rank_fusion(&scores, &hib, 1);
        assert_eq!(result, vec![true, false, false]);
    }

    #[test]
    fn test_borda_count_mixed_polarity() {
        // Signal 0: [10.0, 5.0, 1.0] higher_is_better → ranks [0,1,2]
        // Signal 1: [1.0, 3.0, 2.0]  lower_is_better  → ranks [0,2,1]
        // n=3. Borda(i0)=(3-0)+(3-0)=6, Borda(i1)=(3-1)+(3-2)=3, Borda(i2)=(3-2)+(3-1)=3
        // Top 1 → {0}
        let scores = vec![vec![10.0, 5.0, 1.0], vec![1.0, 3.0, 2.0]];
        let hib = vec![true, false];
        let result = borda_count(&scores, &hib, 1);
        assert_eq!(result, vec![true, false, false]);
    }
}
