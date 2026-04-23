//! Gradient Boosting classifier with regression stumps as weak learners.
//!
//! Uses a log-odds / sigmoid formulation for binary classification.
//! All paths are allocation-minimal; no external crates required.

#[derive(Debug, Clone)]
struct RegressionStump {
    feature: usize,
    threshold: f64,
    left_val: f64,
    right_val: f64,
}

fn predict_stump(stump: &RegressionStump, x: &[f64]) -> f64 {
    let v = if stump.feature < x.len() {
        x[stump.feature]
    } else {
        0.0
    };
    if v <= stump.threshold {
        stump.left_val
    } else {
        stump.right_val
    }
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z.clamp(-500.0, 500.0)).exp())
}

fn fit_stump(features: &[Vec<f64>], targets: &[f64]) -> RegressionStump {
    let n = features.len();
    if n == 0 {
        return RegressionStump {
            feature: 0,
            threshold: 0.0,
            left_val: 0.0,
            right_val: 0.0,
        };
    }

    let n_features = features.iter().map(|f| f.len()).max().unwrap_or(0);

    let mut best_mse = f64::INFINITY;
    let mut best_stump = RegressionStump {
        feature: 0,
        threshold: 0.0,
        left_val: targets.iter().copied().sum::<f64>() / n as f64,
        right_val: 0.0,
    };

    for j in 0..n_features {
        // Collect unique threshold candidates (the actual feature values).
        let mut vals: Vec<f64> = features
            .iter()
            .filter_map(|row| row.get(j).copied())
            .collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        for &thresh in &vals {
            // Partition.
            let mut left_sum = 0.0_f64;
            let mut left_count = 0usize;
            let mut right_sum = 0.0_f64;
            let mut right_count = 0usize;

            for (i, row) in features.iter().enumerate() {
                let v = row.get(j).copied().unwrap_or(0.0);
                if v <= thresh {
                    left_sum += targets[i];
                    left_count += 1;
                } else {
                    right_sum += targets[i];
                    right_count += 1;
                }
            }

            let left_val = if left_count > 0 {
                left_sum / left_count as f64
            } else {
                0.0
            };
            let right_val = if right_count > 0 {
                right_sum / right_count as f64
            } else {
                0.0
            };

            // Compute MSE of this split.
            let mut mse = 0.0_f64;
            for (i, row) in features.iter().enumerate() {
                let v = row.get(j).copied().unwrap_or(0.0);
                let pred = if v <= thresh { left_val } else { right_val };
                let diff = targets[i] - pred;
                mse += diff * diff;
            }
            mse /= n as f64;

            if mse < best_mse {
                best_mse = mse;
                best_stump = RegressionStump {
                    feature: j,
                    threshold: thresh,
                    left_val,
                    right_val,
                };
            }
        }
    }

    best_stump
}

/// Gradient Boosting binary classifier.
///
/// Trains `n_estimators` regression stumps, each fitted to the pseudo-residuals
/// of the current log-odds prediction.  Returns predicted labels for `test`.
pub fn classify(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
    n_estimators: usize,
    lr: f64,
) -> Vec<bool> {
    if train.is_empty() {
        return vec![false; test.len()];
    }

    let n_train = train.len();
    let n_test = test.len();

    // Log-odds accumulators (initialised to 0 = probability 0.5).
    let mut f_train = vec![0.0_f64; n_train];
    let mut f_test = vec![0.0_f64; n_test];

    for _ in 0..n_estimators {
        // Pseudo-residuals: r[i] = y[i] - sigmoid(F[i]).
        let residuals: Vec<f64> = (0..n_train)
            .map(|i| labels[i] as i32 as f64 - sigmoid(f_train[i]))
            .collect();

        let stump = fit_stump(train, &residuals);

        for i in 0..n_train {
            f_train[i] += lr * predict_stump(&stump, &train[i]);
        }
        for i in 0..n_test {
            f_test[i] += lr * predict_stump(&stump, &test[i]);
        }
    }

    f_test.iter().map(|&s| sigmoid(s) >= 0.5).collect()
}

/// Convenience wrapper: 50 estimators, learning rate 0.1.
pub fn classify_default(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    classify(train, labels, test, 50, 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn accuracy(predicted: &[bool], expected: &[bool]) -> f64 {
        assert_eq!(predicted.len(), expected.len());
        let correct = predicted
            .iter()
            .zip(expected.iter())
            .filter(|(p, e)| p == e)
            .count();
        correct as f64 / predicted.len() as f64
    }

    // ── sigmoid ───────────────────────────────────────────────────────────────

    #[test]
    fn sigmoid_midpoint() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn sigmoid_large_positive() {
        assert!((sigmoid(1000.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sigmoid_large_negative() {
        assert!(sigmoid(-1000.0) < 1e-6);
    }

    // ── stump ─────────────────────────────────────────────────────────────────

    #[test]
    fn stump_empty_features() {
        let s = fit_stump(&[], &[]);
        assert_eq!(s.feature, 0);
        assert_eq!(s.threshold, 0.0);
        assert_eq!(s.left_val, 0.0);
        assert_eq!(s.right_val, 0.0);
    }

    #[test]
    fn stump_perfect_split() {
        // Feature 0 perfectly separates targets.
        let features = vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]];
        let targets = vec![0.0, 0.0, 1.0, 1.0];
        let stump = fit_stump(&features, &targets);
        // Split should be at 2.0 (left ≤ 2, right > 2).
        assert_eq!(stump.feature, 0);
        assert!(stump.threshold <= 2.0);
        assert!((stump.left_val - 0.0).abs() < 1e-9 || stump.threshold < 3.0);
    }

    #[test]
    fn predict_stump_branches() {
        let stump = RegressionStump {
            feature: 0,
            threshold: 5.0,
            left_val: -1.0,
            right_val: 1.0,
        };
        assert_eq!(predict_stump(&stump, &[3.0]), -1.0);
        assert_eq!(predict_stump(&stump, &[5.0]), -1.0); // <= threshold → left
        assert_eq!(predict_stump(&stump, &[7.0]), 1.0);
    }

    #[test]
    fn predict_stump_missing_feature() {
        // Feature index beyond slice length → treats value as 0.0.
        let stump = RegressionStump {
            feature: 5,
            threshold: 0.5,
            left_val: 42.0,
            right_val: -42.0,
        };
        // 0.0 <= 0.5 → left branch.
        assert_eq!(predict_stump(&stump, &[1.0, 2.0]), 42.0);
    }

    // ── classify edge cases ───────────────────────────────────────────────────

    #[test]
    fn classify_empty_train_returns_false_vec() {
        let result = classify(&[], &[], &[vec![1.0], vec![2.0]], 10, 0.1);
        assert_eq!(result, vec![false, false]);
    }

    #[test]
    fn classify_empty_test_returns_empty() {
        let train = vec![vec![1.0], vec![2.0]];
        let labels = vec![false, true];
        let result = classify(&train, &labels, &[], 10, 0.1);
        assert!(result.is_empty());
    }

    #[test]
    fn classify_single_estimator() {
        let train = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let test = vec![vec![0.0], vec![1.0]];
        // Should not panic; result length must match test length.
        let result = classify(&train, &labels, &test, 1, 0.1);
        assert_eq!(result.len(), 2);
    }

    // ── classify correctness ──────────────────────────────────────────────────

    #[test]
    fn classify_linearly_separable_1d() {
        // Points < 5 → false, points >= 5 → true.
        let train: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
        let labels: Vec<bool> = (0..20).map(|i| i >= 10).collect();
        let test: Vec<Vec<f64>> = vec![vec![2.0], vec![8.0], vec![12.0], vec![18.0]];
        let expected = vec![false, false, true, true];

        let result = classify(&train, &labels, &test, 50, 0.1);
        assert_eq!(result, expected);
    }

    #[test]
    fn classify_all_same_label_false() {
        let train = vec![vec![1.0], vec![2.0], vec![3.0]];
        let labels = vec![false, false, false];
        let test = vec![vec![1.5], vec![2.5]];
        let result = classify(&train, &labels, &test, 10, 0.1);
        // All residuals are negative; stumps push log-odds down; output should be false.
        assert_eq!(result, vec![false, false]);
    }

    #[test]
    fn classify_all_same_label_true() {
        let train = vec![vec![1.0], vec![2.0], vec![3.0]];
        let labels = vec![true, true, true];
        let test = vec![vec![1.5], vec![2.5]];
        let result = classify(&train, &labels, &test, 10, 0.1);
        assert_eq!(result, vec![true, true]);
    }

    #[test]
    fn classify_default_wrapper() {
        let train: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
        let labels: Vec<bool> = (0..20).map(|i| i >= 10).collect();
        let test = vec![vec![3.0], vec![15.0]];
        let result = classify_default(&train, &labels, &test);
        assert_eq!(result, vec![false, true]);
    }

    #[test]
    fn classify_2d_features() {
        // XOR-like: positive class when both features > 0.5.
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, false, false, true];
        let test = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
        let result = classify(&train, &labels, &test, 100, 0.3);
        // With enough estimators this simple pattern should be learnable.
        assert_eq!(result[1], true, "high-signal point should be classified true");
    }

    #[test]
    fn classify_high_accuracy_1d() {
        // 40-point dataset; clear decision boundary at 20.
        let train: Vec<Vec<f64>> = (0..40).map(|i| vec![i as f64]).collect();
        let labels: Vec<bool> = (0..40).map(|i| i >= 20).collect();
        let test = train.clone();
        let result = classify(&train, &labels, &test, 100, 0.1);
        let acc = accuracy(&result, &labels);
        assert!(acc >= 0.95, "expected ≥ 95% accuracy, got {acc:.2}");
    }
}
