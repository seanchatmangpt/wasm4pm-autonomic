/// Lloyd's algorithm. Returns cluster assignment (0..k) for each input point.
/// Deterministic seeding: centroids initialized at indices 0, n/k, 2n/k, ...
pub fn cluster(features: &[Vec<f64>], k: usize, max_iter: usize) -> Vec<usize> {
    if k == 0 || features.is_empty() {
        return vec![];
    }

    let n = features.len();
    let k = k.min(n);
    let dim = features[0].len();

    // Initialize centroids at evenly spaced indices.
    let mut centroids: Vec<Vec<f64>> = (0..k)
        .map(|c| features[c * n / k].clone())
        .collect();

    let mut assignments = vec![0usize; n];
    let mut prev_assignments = vec![usize::MAX; n];

    for _ in 0..max_iter {
        // Assign each point to the nearest centroid.
        for (i, point) in features.iter().enumerate() {
            let mut best_cluster = 0;
            let mut best_dist = f64::INFINITY;
            for (c, centroid) in centroids.iter().enumerate() {
                let dist = squared_euclidean(point, centroid);
                // Treat NaN distance as infinity — falls through to cluster 0.
                if dist < best_dist {
                    best_dist = dist;
                    best_cluster = c;
                }
            }
            assignments[i] = best_cluster;
        }

        // Check for convergence.
        if assignments == prev_assignments {
            break;
        }
        prev_assignments.clone_from(&assignments);

        // Recompute centroids as mean of assigned points.
        let mut sums = vec![vec![0.0f64; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, point) in features.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for d in 0..dim {
                let v = point[d];
                // Treat NaN as 0.
                sums[c][d] += if v.is_nan() { 0.0 } else { v };
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                let count = counts[c] as f64;
                for d in 0..dim {
                    centroids[c][d] = sums[c][d] / count;
                }
            }
            // If cluster is empty, keep old centroid.
        }
    }

    assignments
}

/// Run k=2 k-means, then label clusters using seed_labels hints.
/// `seed_labels[i] = Some(true)` means trace i is known positive (from in_language BFS).
/// Returns bool classification for all traces.
pub fn classify_unsupervised(features: &[Vec<f64>], seed_labels: &[Option<bool>]) -> Vec<bool> {
    if features.is_empty() {
        return vec![];
    }

    let assignments = cluster(features, 2, 100);

    // Count how many Some(true) seeds fall in cluster 0 vs cluster 1.
    let mut positive_count = [0usize; 2];
    for (i, label) in seed_labels.iter().enumerate() {
        if let Some(true) = label {
            if i < assignments.len() {
                positive_count[assignments[i]] += 1;
            }
        }
    }

    // Cluster with more Some(true) seeds → "positive" cluster.
    // Tie-break: cluster 0 = negative (conservative), so cluster 1 wins ties.
    let positive_cluster = if positive_count[1] > positive_count[0] {
        1
    } else {
        // cluster 0 has more or equal positives; but tie → cluster 0 is negative,
        // meaning if equal we still need a consistent rule.
        // Per spec: tie-break → cluster 0 = negative, so positive cluster = 0
        // only when cluster 0 strictly wins.
        if positive_count[0] > positive_count[1] {
            0
        } else {
            // True tie: cluster 0 = negative → no cluster is "positive" by seeds.
            // Default: cluster 1 = positive to pick a deterministic side.
            // Spec says "cluster 0 = negative (conservative)" on tie, so positive = 1.
            1
        }
    };

    assignments
        .iter()
        .map(|&c| c == positive_cluster)
        .collect()
}

#[inline]
fn squared_euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = x - y;
            if d.is_nan() { 0.0 } else { d * d }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_empty() {
        assert_eq!(cluster(&[], 3, 10), vec![]);
    }

    #[test]
    fn test_cluster_k_zero() {
        let features = vec![vec![1.0, 2.0]];
        assert_eq!(cluster(&features, 0, 10), vec![]);
    }

    #[test]
    fn test_cluster_k_clamped() {
        // 2 points, k=5 → clamped to 2
        let features = vec![vec![0.0], vec![10.0]];
        let result = cluster(&features, 5, 100);
        assert_eq!(result.len(), 2);
        assert_ne!(result[0], result[1]);
    }

    #[test]
    fn test_cluster_two_clear_groups() {
        // Two well-separated clusters
        let features = vec![
            vec![0.0, 0.0],
            vec![0.1, 0.1],
            vec![10.0, 10.0],
            vec![10.1, 10.1],
        ];
        let result = cluster(&features, 2, 100);
        assert_eq!(result.len(), 4);
        // First two should be in same cluster, last two in the other.
        assert_eq!(result[0], result[1]);
        assert_eq!(result[2], result[3]);
        assert_ne!(result[0], result[2]);
    }

    #[test]
    fn test_classify_unsupervised_empty() {
        let result = classify_unsupervised(&[], &[]);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_classify_unsupervised_labels_positive_cluster() {
        let features = vec![
            vec![0.0],
            vec![0.1],
            vec![10.0],
            vec![10.1],
        ];
        // Mark last two as known positive.
        let seeds = vec![None, None, Some(true), Some(true)];
        let result = classify_unsupervised(&features, &seeds);
        assert_eq!(result.len(), 4);
        assert!(!result[0]);
        assert!(!result[1]);
        assert!(result[2]);
        assert!(result[3]);
    }

    #[test]
    fn test_classify_unsupervised_no_seeds() {
        // No seeds → deterministic but just check it runs without panic.
        let features = vec![vec![0.0], vec![1.0]];
        let seeds: Vec<Option<bool>> = vec![None, None];
        let result = classify_unsupervised(&features, &seeds);
        assert_eq!(result.len(), 2);
    }
}
