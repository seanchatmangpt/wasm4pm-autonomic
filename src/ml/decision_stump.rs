/// Decision Stump — depth-1 binary decision tree classifier.
///
/// Splits on a single feature threshold to minimize training error.
/// All operations are allocation-minimal on the hot path; only the
/// candidate-generation phase allocates temporary `Vec`s.
#[derive(Debug, Clone)]
pub struct Stump {
    /// Index into the feature vector used for splitting.
    pub feature: usize,
    /// Threshold value for the split.
    pub threshold: f64,
    /// Predicted label for examples where `x[feature] <= threshold`.
    pub left_label: bool,
    /// Predicted label for examples where `x[feature] > threshold`.
    pub right_label: bool,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the majority label among `labels`, defaulting to `false` on a tie.
fn majority(labels: &[bool]) -> bool {
    if labels.is_empty() {
        return false;
    }
    let trues = labels.iter().filter(|&&l| l).count();
    trues * 2 >= labels.len() // >= so tie goes to `true` (stable, deterministic)
}

/// Accuracy of a single (feature, threshold, left_label, right_label) split
/// over the full training set.
fn accuracy(
    train: &[Vec<f64>],
    labels: &[bool],
    feature: usize,
    threshold: f64,
    left_label: bool,
    right_label: bool,
) -> f64 {
    let correct = train
        .iter()
        .zip(labels.iter())
        .filter(|(x, &label)| {
            let predicted = if x[feature] <= threshold {
                left_label
            } else {
                right_label
            };
            predicted == label
        })
        .count();
    correct as f64 / train.len() as f64
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fit a [`Stump`] to labeled binary training data.
///
/// # Edge cases
/// - Empty `train` or zero features → returns a zeroed stump.
/// - Single training example → `left_label == right_label == that example's label`.
/// - All values of a feature are identical → `threshold = f64::INFINITY`,
///   `left_label = majority(all labels)`.
pub fn fit(train: &[Vec<f64>], labels: &[bool]) -> Stump {
    // Guard: empty or no features.
    if train.is_empty() || train[0].is_empty() {
        return Stump {
            feature: 0,
            threshold: 0.0,
            left_label: false,
            right_label: false,
        };
    }

    let n_features = train[0].len();
    let n = train.len();

    // Single example shortcut.
    if n == 1 {
        let label = labels[0];
        return Stump {
            feature: 0,
            threshold: 0.0,
            left_label: label,
            right_label: label,
        };
    }

    let mut best_acc = -1.0_f64;
    let mut best = Stump {
        feature: 0,
        threshold: 0.0,
        left_label: false,
        right_label: false,
    };

    for j in 0..n_features {
        // Collect feature values paired with labels, then sort.
        let mut vals: Vec<f64> = train.iter().map(|x| x[j]).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

        // Build candidate threshold list.
        let mut thresholds: Vec<f64> = Vec::with_capacity(vals.len() + 2);

        if vals.len() == 1 {
            // All feature values identical — send every example to the left.
            thresholds.push(f64::INFINITY);
        } else {
            // Midpoints between consecutive unique values.
            for k in 0..vals.len() - 1 {
                thresholds.push((vals[k] + vals[k + 1]) / 2.0);
            }
            // Sentinel: everything goes right (left bucket empty).
            thresholds.push(vals[0] - 1.0);
            // Sentinel: everything goes left (right bucket empty).
            thresholds.push(vals[vals.len() - 1] + 1.0);
        }

        for &thresh in &thresholds {
            // Partition labels.
            let left_labels: Vec<bool> = train
                .iter()
                .zip(labels.iter())
                .filter(|(x, _)| x[j] <= thresh)
                .map(|(_, &l)| l)
                .collect();
            let right_labels: Vec<bool> = train
                .iter()
                .zip(labels.iter())
                .filter(|(x, _)| x[j] > thresh)
                .map(|(_, &l)| l)
                .collect();

            let ll = majority(&left_labels);
            let rl = majority(&right_labels);

            let acc = accuracy(train, labels, j, thresh, ll, rl);

            // Strict `>` preserves tie-break by lower feature index (we iterate j ascending).
            if acc > best_acc {
                best_acc = acc;
                best = Stump {
                    feature: j,
                    threshold: thresh,
                    left_label: ll,
                    right_label: rl,
                };
            }
        }
    }

    best
}

/// Apply a fitted [`Stump`] to a slice of feature vectors.
pub fn predict(stump: &Stump, test: &[Vec<f64>]) -> Vec<bool> {
    test.iter()
        .map(|x| {
            if x[stump.feature] <= stump.threshold {
                stump.left_label
            } else {
                stump.right_label
            }
        })
        .collect()
}

/// Convenience: fit on `train`/`labels` then predict on `test`.
pub fn classify(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    let stump = fit(train, labels);
    predict(&stump, test)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_train() -> (Vec<Vec<f64>>, Vec<bool>) {
        // Simple linearly separable 1-D data.
        let train = vec![
            vec![1.0],
            vec![2.0],
            vec![3.0],
            vec![6.0],
            vec![7.0],
            vec![8.0],
        ];
        let labels = vec![false, false, false, true, true, true];
        (train, labels)
    }

    #[test]
    fn test_fit_perfect_split() {
        let (train, labels) = make_train();
        let stump = fit(&train, &labels);
        assert_eq!(stump.feature, 0);
        // Threshold should separate 3.0 from 6.0, e.g. 4.5
        assert!(stump.threshold > 3.0 && stump.threshold < 6.0);
        assert_eq!(stump.left_label, false);
        assert_eq!(stump.right_label, true);
    }

    #[test]
    fn test_predict() {
        let (train, labels) = make_train();
        let stump = fit(&train, &labels);
        let preds = predict(&stump, &[vec![0.5], vec![9.0]]);
        assert_eq!(preds, vec![false, true]);
    }

    #[test]
    fn test_classify() {
        let (train, labels) = make_train();
        let preds = classify(&train, &labels, &[vec![2.5], vec![6.5]]);
        assert_eq!(preds, vec![false, true]);
    }

    #[test]
    fn test_empty_train() {
        let stump = fit(&[], &[]);
        assert_eq!(stump.feature, 0);
        assert!((stump.threshold - 0.0).abs() < f64::EPSILON);
        assert_eq!(stump.left_label, false);
        assert_eq!(stump.right_label, false);
    }

    #[test]
    fn test_single_example() {
        let train = vec![vec![3.0, 7.0]];
        let labels = vec![true];
        let stump = fit(&train, &labels);
        assert_eq!(stump.left_label, true);
        assert_eq!(stump.right_label, true);
    }

    #[test]
    fn test_all_identical_feature() {
        let train = vec![vec![5.0], vec![5.0], vec![5.0]];
        let labels = vec![true, false, true];
        let stump = fit(&train, &labels);
        // All values identical → threshold = INFINITY, everything goes left.
        assert_eq!(stump.threshold, f64::INFINITY);
        // Majority of [true, false, true] = true.
        assert_eq!(stump.left_label, true);
    }

    #[test]
    fn test_multi_feature_selects_best() {
        // Feature 0 is useless (all same), feature 1 splits perfectly.
        let train = vec![
            vec![1.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, false, true, true];
        let stump = fit(&train, &labels);
        assert_eq!(stump.feature, 1);
        assert_eq!(stump.left_label, false);
        assert_eq!(stump.right_label, true);
    }

    #[test]
    fn test_majority_tie_goes_to_true() {
        // Equal split: tie → true.
        assert_eq!(majority(&[true, false]), true);
        assert_eq!(majority(&[false, true]), true);
    }

    #[test]
    fn test_majority_empty() {
        assert_eq!(majority(&[]), false);
    }
}
