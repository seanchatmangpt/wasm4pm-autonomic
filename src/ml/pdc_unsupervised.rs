//! Unsupervised classifier pipeline for PDC 2025.
//!
//! Combines k-means, agglomerative clustering, fitness-rank, and in-language fill
//! strategies to produce binary (positive/negative) trace labels without requiring
//! labelled training data. `seed_labels` from in_language BFS results act as weak
//! supervision hints for cluster polarity assignment.

use crate::ml::{hierarchical_clustering, kmeans};
use crate::ml::hierarchical_clustering::Linkage;

/// All unsupervised binary predictions for a set of traces.
#[derive(Debug, Default, Clone)]
pub struct UnsupervisedPredictions {
    /// k-means (k=2) classification, polarity guided by seed_labels.
    pub kmeans: Vec<bool>,
    /// 2-cluster agglomerative (single linkage), polarity guided by seed_labels.
    pub hierarchical_single: Vec<bool>,
    /// 2-cluster agglomerative (complete linkage), polarity guided by seed_labels.
    pub hierarchical_complete: Vec<bool>,
    /// 2-cluster agglomerative (average linkage), polarity guided by seed_labels.
    pub hierarchical_average: Vec<bool>,
    /// Fitness-rank-only: top `n_target` traces by fitness score (descending), tie-break by index.
    pub fitness_rank: Vec<bool>,
    /// in_language + fitness fill: confirmed positives first, fill remaining slots to
    /// `n_target` by fitness descending among non-in_lang traces.
    pub in_lang_fill: Vec<bool>,
}

/// Run all unsupervised classifiers and return their predictions.
///
/// # Arguments
/// * `features`     – Feature vectors for each trace (one `Vec<f64>` per trace).
/// * `seed_labels`  – in_language BFS results: `Some(true)` = confirmed positive,
///                    `Some(false)` = confirmed negative, `None` = unknown.
/// * `fitness`      – Token-replay fitness score per trace.
/// * `n_target`     – Number of positives to select (500 for PDC 2025).
///
/// # Performance guard
/// Agglomerative clustering is O(n³). When `features.len() > 200` the hierarchical
/// variants fall back to the k-means result to keep the pipeline tractable.
pub fn run_unsupervised(
    features: &[Vec<f64>],
    seed_labels: &[Option<bool>],
    fitness: &[f64],
    n_target: usize,
) -> UnsupervisedPredictions {
    let n = features.len();

    // --- k-means -----------------------------------------------------------
    let kmeans_preds = kmeans::classify_unsupervised(features, seed_labels);

    // --- hierarchical (with size guard) ------------------------------------
    let use_hierarchical = n <= 200;

    let hierarchical_single = if use_hierarchical {
        hierarchical_clustering::classify_unsupervised(features, seed_labels, Linkage::Single)
    } else {
        kmeans_preds.clone()
    };

    let hierarchical_complete = if use_hierarchical {
        hierarchical_clustering::classify_unsupervised(features, seed_labels, Linkage::Complete)
    } else {
        kmeans_preds.clone()
    };

    let hierarchical_average = if use_hierarchical {
        hierarchical_clustering::classify_unsupervised(features, seed_labels, Linkage::Average)
    } else {
        kmeans_preds.clone()
    };

    // --- fitness rank -------------------------------------------------------
    // Sort indices by fitness descending; tie-break by original index ascending.
    let fitness_rank = fitness_rank_labels(fitness, n_target);

    // --- in_lang fill -------------------------------------------------------
    let in_lang_fill = in_lang_fill_labels(seed_labels, fitness, n_target);

    UnsupervisedPredictions {
        kmeans: kmeans_preds,
        hierarchical_single,
        hierarchical_complete,
        hierarchical_average,
        fitness_rank,
        in_lang_fill,
    }
}

/// Return a `Vec<(&'static str, Vec<bool>)>` suitable for iteration / reporting.
pub fn to_named_list(preds: &UnsupervisedPredictions) -> Vec<(&'static str, Vec<bool>)> {
    vec![
        ("kmeans", preds.kmeans.clone()),
        ("hierarchical_single", preds.hierarchical_single.clone()),
        ("hierarchical_complete", preds.hierarchical_complete.clone()),
        ("hierarchical_average", preds.hierarchical_average.clone()),
        ("fitness_rank", preds.fitness_rank.clone()),
        ("in_lang_fill", preds.in_lang_fill.clone()),
    ]
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Top-`n_target` traces by fitness score → `true`.
/// Tie-break: lower original index wins (appears earlier in sorted order).
fn fitness_rank_labels(fitness: &[f64], n_target: usize) -> Vec<bool> {
    let n = fitness.len();
    if n == 0 {
        return vec![];
    }

    // Build index list sorted by fitness descending, then by index ascending.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        // Descending fitness; NaN sorts last.
        fitness[b]
            .partial_cmp(&fitness[a])
            .unwrap_or(std::cmp::Ordering::Greater)
            .then(a.cmp(&b)) // tie-break: lower index first
    });

    let mut labels = vec![false; n];
    for &idx in order.iter().take(n_target) {
        labels[idx] = true;
    }
    labels
}

/// in_language confirmed positives get `true` first; remaining slots filled by
/// fitness descending (among traces that are not already confirmed positive).
fn in_lang_fill_labels(
    seed_labels: &[Option<bool>],
    fitness: &[f64],
    n_target: usize,
) -> Vec<bool> {
    let n = seed_labels.len();
    if n == 0 {
        return vec![];
    }

    let mut labels = vec![false; n];

    // Pass 1: mark all confirmed positives.
    let mut confirmed_count = 0usize;
    for (i, label) in seed_labels.iter().enumerate() {
        if matches!(label, Some(true)) {
            labels[i] = true;
            confirmed_count += 1;
        }
    }

    // Pass 2: if we still have slots, fill by fitness descending among non-confirmed.
    let remaining = n_target.saturating_sub(confirmed_count);
    if remaining > 0 {
        // Collect non-confirmed indices sorted by fitness descending, tie-break by index.
        let fitness_len = fitness.len();
        let mut fill_candidates: Vec<usize> = (0..n)
            .filter(|&i| !labels[i])
            .filter(|&i| i < fitness_len) // guard: fitness slice may be shorter
            .collect();

        fill_candidates.sort_by(|&a, &b| {
            fitness[b]
                .partial_cmp(&fitness[a])
                .unwrap_or(std::cmp::Ordering::Greater)
                .then(a.cmp(&b))
        });

        for &idx in fill_candidates.iter().take(remaining) {
            labels[idx] = true;
        }
    }

    labels
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_features(vals: &[f64]) -> Vec<Vec<f64>> {
        vals.iter().map(|&v| vec![v]).collect()
    }

    // -----------------------------------------------------------------------
    // fitness_rank_labels
    // -----------------------------------------------------------------------

    #[test]
    fn test_fitness_rank_empty() {
        assert!(fitness_rank_labels(&[], 5).is_empty());
    }

    #[test]
    fn test_fitness_rank_selects_top_n() {
        // fitness = [0.1, 0.9, 0.5, 0.8, 0.3]; top-2 = indices 1, 3
        let fitness = vec![0.1, 0.9, 0.5, 0.8, 0.3];
        let labels = fitness_rank_labels(&fitness, 2);
        assert_eq!(labels.len(), 5);
        assert!(!labels[0]);
        assert!(labels[1]);
        assert!(!labels[2]);
        assert!(labels[3]);
        assert!(!labels[4]);
    }

    #[test]
    fn test_fitness_rank_tie_break_by_index() {
        // All equal fitness — tie-break: lower index wins; n_target=2 → indices 0,1
        let fitness = vec![0.5, 0.5, 0.5, 0.5];
        let labels = fitness_rank_labels(&fitness, 2);
        assert!(labels[0]);
        assert!(labels[1]);
        assert!(!labels[2]);
        assert!(!labels[3]);
    }

    #[test]
    fn test_fitness_rank_n_target_exceeds_n() {
        // n_target > n → all true
        let fitness = vec![0.3, 0.7];
        let labels = fitness_rank_labels(&fitness, 10);
        assert!(labels.iter().all(|&l| l));
    }

    // -----------------------------------------------------------------------
    // in_lang_fill_labels
    // -----------------------------------------------------------------------

    #[test]
    fn test_in_lang_fill_empty() {
        assert!(in_lang_fill_labels(&[], &[], 5).is_empty());
    }

    #[test]
    fn test_in_lang_fill_confirmed_positives_always_true() {
        // 3 confirmed positives, n_target=2; confirmed still all marked true.
        let seeds = vec![Some(true), Some(true), Some(true), None, None];
        let fitness = vec![0.1, 0.1, 0.1, 0.9, 0.8];
        let labels = in_lang_fill_labels(&seeds, &fitness, 2);
        // All 3 confirmed positives are true regardless of n_target.
        assert!(labels[0]);
        assert!(labels[1]);
        assert!(labels[2]);
        // No fill needed (3 > 2), so unknowns stay false.
        assert!(!labels[3]);
        assert!(!labels[4]);
    }

    #[test]
    fn test_in_lang_fill_fills_by_fitness() {
        // 1 confirmed positive, n_target=3 → fill 2 more by fitness from {1,2,3,4}.
        // fitness: idx 3 = 0.95, idx 4 = 0.85, idx 1 = 0.5, idx 2 = 0.1
        let seeds = vec![Some(true), None, None, None, None];
        let fitness = vec![0.0, 0.5, 0.1, 0.95, 0.85];
        let labels = in_lang_fill_labels(&seeds, &fitness, 3);
        assert!(labels[0]); // confirmed positive
        assert!(!labels[1]); // fitness 0.5 → 3rd among non-confirmed; not picked
        assert!(!labels[2]); // fitness 0.1 → not picked
        assert!(labels[3]); // fitness 0.95 → top fill
        assert!(labels[4]); // fitness 0.85 → second fill
    }

    // -----------------------------------------------------------------------
    // run_unsupervised
    // -----------------------------------------------------------------------

    #[test]
    fn test_run_unsupervised_empty() {
        let preds = run_unsupervised(&[], &[], &[], 5);
        assert!(preds.kmeans.is_empty());
        assert!(preds.hierarchical_single.is_empty());
        assert!(preds.hierarchical_complete.is_empty());
        assert!(preds.hierarchical_average.is_empty());
        assert!(preds.fitness_rank.is_empty());
        assert!(preds.in_lang_fill.is_empty());
    }

    #[test]
    fn test_run_unsupervised_small_uses_hierarchical() {
        // 6 points ≤ 200 → hierarchical runs for real (not kmeans fallback).
        // Two clear groups: [0,1,2] vs [100,101,102]; seed positives in second group.
        let features = make_features(&[0.0, 1.0, 2.0, 100.0, 101.0, 102.0]);
        let seeds: Vec<Option<bool>> = vec![None, None, None, Some(true), Some(true), None];
        let fitness = vec![0.1, 0.2, 0.3, 0.8, 0.9, 0.7];

        let preds = run_unsupervised(&features, &seeds, &fitness, 3);

        assert_eq!(preds.kmeans.len(), 6);
        assert_eq!(preds.hierarchical_single.len(), 6);
        assert_eq!(preds.hierarchical_complete.len(), 6);
        assert_eq!(preds.hierarchical_average.len(), 6);
        assert_eq!(preds.fitness_rank.len(), 6);
        assert_eq!(preds.in_lang_fill.len(), 6);

        // fitness_rank top-3: indices 4 (0.9), 3 (0.8), 5 (0.7)
        assert!(preds.fitness_rank[4]);
        assert!(preds.fitness_rank[3]);
        assert!(preds.fitness_rank[5]);
        assert!(!preds.fitness_rank[0]);
        assert!(!preds.fitness_rank[1]);
        assert!(!preds.fitness_rank[2]);
    }

    #[test]
    fn test_run_unsupervised_large_falls_back_to_kmeans() {
        // 201 points > 200 → hierarchical fields equal kmeans.
        let features: Vec<Vec<f64>> = (0..201).map(|i| vec![i as f64]).collect();
        let seeds: Vec<Option<bool>> = (0..201)
            .map(|i| if i >= 150 { Some(true) } else { None })
            .collect();
        let fitness: Vec<f64> = (0..201).map(|i| i as f64 / 200.0).collect();

        let preds = run_unsupervised(&features, &seeds, &fitness, 100);

        assert_eq!(preds.hierarchical_single, preds.kmeans);
        assert_eq!(preds.hierarchical_complete, preds.kmeans);
        assert_eq!(preds.hierarchical_average, preds.kmeans);
    }

    // -----------------------------------------------------------------------
    // to_named_list
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_named_list_names_and_count() {
        let preds = UnsupervisedPredictions {
            kmeans: vec![true, false],
            hierarchical_single: vec![true, false],
            hierarchical_complete: vec![false, true],
            hierarchical_average: vec![true, true],
            fitness_rank: vec![false, false],
            in_lang_fill: vec![true, false],
        };
        let list = to_named_list(&preds);
        assert_eq!(list.len(), 6);
        let names: Vec<&str> = list.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"kmeans"));
        assert!(names.contains(&"hierarchical_single"));
        assert!(names.contains(&"hierarchical_complete"));
        assert!(names.contains(&"hierarchical_average"));
        assert!(names.contains(&"fitness_rank"));
        assert!(names.contains(&"in_lang_fill"));
        // Each vec has 2 elements.
        for (_, v) in &list {
            assert_eq!(v.len(), 2);
        }
    }
}
