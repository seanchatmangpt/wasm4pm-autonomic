// k-Nearest Neighbors classifier — from scratch, no external ML crates
// Euclidean distance, majority vote among k nearest neighbors

/// Classify each test point using k-NN against the labeled training set.
///
/// - Distance metric: Euclidean (handles empty feature vectors as distance 0.0)
/// - Decision rule: majority vote among the k nearest; tie-breaks toward `false`
/// - Edge cases:
///   - Empty training set → every test point returns `false`
///   - `k == 0` → treated as `k = 1`
///   - `k > train.len()` → clamped to `train.len()`
pub fn classify(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>], k: usize) -> Vec<bool> {
    if train.is_empty() || k == 0 {
        return vec![false; test.len()];
    }

    let k_eff = k.min(train.len());

    test.iter()
        .map(|test_point| {
            // Compute distances from this test point to every training point.
            let mut distances: Vec<(f64, bool)> = train
                .iter()
                .zip(labels.iter())
                .map(|(train_point, &label)| {
                    let dist = euclidean(test_point, train_point);
                    (dist, label)
                })
                .collect();

            // Sort by distance ascending; NaN-safe via unwrap_or(Equal).
            distances.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Majority vote among k_eff nearest neighbors.
            let true_votes = distances[..k_eff]
                .iter()
                .filter(|&&(_, label)| label)
                .count();

            // Tie-break toward false: strictly more than half must vote true.
            true_votes * 2 > k_eff
        })
        .collect()
}

/// Convenience wrapper with `k = 3`.
pub fn classify_k3(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    classify(train, labels, test, 3)
}

/// Euclidean distance between two feature vectors.
///
/// If the vectors differ in length the shorter one is zero-padded implicitly
/// (extra dimensions contribute their squared value directly).
/// An empty pair of vectors returns `0.0`.
fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 0.0;
    }

    let sum_sq: f64 = (0..max_len)
        .map(|i| {
            let ai = a.get(i).copied().unwrap_or(0.0);
            let bi = b.get(i).copied().unwrap_or(0.0);
            let diff = ai - bi;
            diff * diff
        })
        .sum();

    sum_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_train_returns_all_false() {
        let result = classify(&[], &[], &[vec![1.0, 2.0]], 3);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_k_zero_returns_all_false() {
        let train = vec![vec![1.0], vec![2.0]];
        let labels = vec![true, false];
        let test = vec![vec![1.0]];
        let result = classify(&train, &labels, &test, 0);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_k_clamped_to_train_len() {
        let train = vec![vec![0.0], vec![1.0], vec![2.0]];
        let labels = vec![true, true, false];
        let test = vec![vec![0.5]];
        // k=100 clamped to 3; majority is true (2 of 3)
        let result = classify(&train, &labels, &test, 100);
        assert_eq!(result, vec![true]);
    }

    #[test]
    fn test_simple_1d_classification() {
        // Points at 0.0 and 10.0; test at 1.0 → nearest is 0.0 (false)
        let train = vec![vec![0.0], vec![10.0]];
        let labels = vec![false, true];
        let test = vec![vec![1.0], vec![9.0]];
        let result = classify(&train, &labels, &test, 1);
        assert_eq!(result, vec![false, true]);
    }

    #[test]
    fn test_tie_breaks_toward_false() {
        // k=2, one true and one false neighbor — should return false
        let train = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let test = vec![vec![0.5]];
        let result = classify(&train, &labels, &test, 2);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_empty_feature_vectors() {
        // Both train and test have empty feature vectors → distance = 0.0
        let train = vec![vec![], vec![]];
        let labels = vec![false, true];
        let test = vec![vec![]];
        // Both equidistant; k=1 picks first (false); k=2 ties → false
        let result = classify(&train, &labels, &test, 1);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_empty_test_set() {
        let train = vec![vec![1.0]];
        let labels = vec![true];
        let result = classify(&train, &labels, &[], 1);
        assert!(result.is_empty());
    }

    #[test]
    fn test_classify_k3_convenience() {
        let train = vec![vec![0.0], vec![0.1], vec![0.2], vec![10.0]];
        let labels = vec![false, false, false, true];
        let test = vec![vec![0.05]];
        // Three nearest are all false
        let result = classify_k3(&train, &labels, &test);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_euclidean_zero_for_identical_points() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(euclidean(&a, &b), 0.0);
    }

    #[test]
    fn test_euclidean_known_distance() {
        // 3-4-5 right triangle
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let d = euclidean(&a, &b);
        assert!((d - 5.0).abs() < 1e-10);
    }
}
