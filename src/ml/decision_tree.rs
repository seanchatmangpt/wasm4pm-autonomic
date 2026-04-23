//! ID3-style Decision Tree classifier with entropy-based information gain.
//!
//! Uses binary splits on the median of each feature. All hot-path allocations
//! are bounded by the training set size and tree depth — no unbounded growth.

/// Internal tree node.
enum Node {
    Leaf(bool),
    Split {
        feature: usize,
        threshold: f64,
        left: Box<Node>,
        right: Box<Node>,
    },
}

// ---------------------------------------------------------------------------
// Entropy helpers
// ---------------------------------------------------------------------------

/// Binary entropy: H = -p·log₂(p) - (1-p)·log₂(1-p).  Returns 0.0 for p∈{0,1}.
#[inline]
fn entropy(pos: usize, total: usize) -> f64 {
    if total == 0 || pos == 0 || pos == total {
        return 0.0;
    }
    let p = pos as f64 / total as f64;
    let q = 1.0 - p;
    -(p * p.log2()) - (q * q.log2())
}

/// Majority-vote label for a slice of (feature_row, label) pairs.
fn majority(labels: &[bool]) -> bool {
    let pos: usize = labels.iter().filter(|&&l| l).count();
    pos * 2 >= labels.len() // ties → true
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Build a decision tree recursively.
///
/// `indices` contains the subset of training rows to consider at this node.
/// We avoid cloning the full training matrix by working with index slices.
fn build(
    train: &[Vec<f64>],
    labels: &[bool],
    indices: &[usize],
    depth: usize,
) -> Node {
    // --- base cases ---
    if depth == 0 || indices.len() < 2 {
        let subset: Vec<bool> = indices.iter().map(|&i| labels[i]).collect();
        return Node::Leaf(majority(&subset));
    }

    let subset_labels: Vec<bool> = indices.iter().map(|&i| labels[i]).collect();

    // All same label?
    let pos_count = subset_labels.iter().filter(|&&l| l).count();
    if pos_count == 0 || pos_count == subset_labels.len() {
        return Node::Leaf(majority(&subset_labels));
    }

    let n = indices.len();
    let parent_entropy = entropy(pos_count, n);

    // Number of features (guard against empty rows)
    let n_features = train.get(indices[0]).map_or(0, |r| r.len());
    if n_features == 0 {
        return Node::Leaf(majority(&subset_labels));
    }

    // --- find best split ---
    let mut best_ig: f64 = 0.0;
    let mut best_feature: usize = 0;
    let mut best_threshold: f64 = 0.0;
    let mut found = false;

    for j in 0..n_features {
        // Collect feature values for current indices, compute median.
        let mut vals: Vec<f64> = indices.iter().map(|&i| train[i][j]).collect();
        vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let threshold = if n % 2 == 1 {
            vals[n / 2]
        } else {
            (vals[n / 2 - 1] + vals[n / 2]) / 2.0
        };

        // Partition.
        let mut left_pos = 0usize;
        let mut left_n = 0usize;
        let mut right_pos = 0usize;
        let mut right_n = 0usize;

        for (&i, &lbl) in indices.iter().zip(subset_labels.iter()) {
            if train[i][j] <= threshold {
                left_n += 1;
                if lbl {
                    left_pos += 1;
                }
            } else {
                right_n += 1;
                if lbl {
                    right_pos += 1;
                }
            }
        }

        // Skip degenerate splits (all on one side).
        if left_n == 0 || right_n == 0 {
            continue;
        }

        let ig = parent_entropy
            - (left_n as f64 / n as f64) * entropy(left_pos, left_n)
            - (right_n as f64 / n as f64) * entropy(right_pos, right_n);

        if ig > best_ig {
            best_ig = ig;
            best_feature = j;
            best_threshold = threshold;
            found = true;
        }
    }

    if !found {
        return Node::Leaf(majority(&subset_labels));
    }

    // Partition indices.
    let (left_idx, right_idx): (Vec<usize>, Vec<usize>) =
        indices.iter().partition(|&&i| train[i][best_feature] <= best_threshold);

    // Guard: both sides must be non-empty (already checked above, but be safe).
    if left_idx.is_empty() || right_idx.is_empty() {
        return Node::Leaf(majority(&subset_labels));
    }

    let left = Box::new(build(train, labels, &left_idx, depth - 1));
    let right = Box::new(build(train, labels, &right_idx, depth - 1));

    Node::Split {
        feature: best_feature,
        threshold: best_threshold,
        left,
        right,
    }
}

// ---------------------------------------------------------------------------
// Predict
// ---------------------------------------------------------------------------

fn predict_one(node: &Node, row: &[f64]) -> bool {
    let mut current = node;
    loop {
        match current {
            Node::Leaf(label) => return *label,
            Node::Split {
                feature,
                threshold,
                left,
                right,
            } => {
                let val = row.get(*feature).copied().unwrap_or(0.0);
                if val <= *threshold {
                    current = left;
                } else {
                    current = right;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Train an ID3 decision tree on `(train, labels)` and classify each row in
/// `test`.  Splits are chosen by maximum information gain using binary entropy;
/// the threshold for each feature is the median of that feature in the current
/// node's sample.
///
/// Returns a `Vec<bool>` of predicted labels (one per test row).
///
/// # Panics
/// Never panics; gracefully handles empty slices and mismatched lengths.
pub fn classify(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
    max_depth: usize,
) -> Vec<bool> {
    if train.is_empty() || labels.is_empty() || test.is_empty() {
        return vec![false; test.len()];
    }

    let n = train.len().min(labels.len());
    let indices: Vec<usize> = (0..n).collect();

    let root = build(train, labels, &indices, max_depth);
    test.iter().map(|row| predict_one(&root, row)).collect()
}

/// Convenience wrapper with `max_depth = 3`.
pub fn classify_d3(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    classify(train, labels, test, 3)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn xor_dataset() -> (Vec<Vec<f64>>, Vec<bool>) {
        // Simple 2D XOR-like dataset.
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, true, true, false, false, false];
        (train, labels)
    }

    #[test]
    fn test_entropy_edge_cases() {
        assert_eq!(entropy(0, 10), 0.0);
        assert_eq!(entropy(10, 10), 0.0);
        // Balanced: entropy should be ~1.0
        let e = entropy(5, 10);
        assert!((e - 1.0).abs() < 1e-10, "expected ~1.0 got {}", e);
    }

    #[test]
    fn test_majority_tie_goes_true() {
        // Even split → true
        assert!(majority(&[true, false]));
        assert!(majority(&[true]));
        assert!(!majority(&[false]));
        assert!(!majority(&[false, false, true]));
    }

    #[test]
    fn test_trivial_linearly_separable() {
        // All positives in x[0] > 0.5
        let train = vec![
            vec![0.0], vec![0.0], vec![1.0], vec![1.0],
        ];
        let labels = vec![false, false, true, true];
        let test = vec![vec![0.0], vec![1.0]];
        let preds = classify(&train, &labels, &test, 3);
        assert_eq!(preds, vec![false, true]);
    }

    #[test]
    fn test_empty_train_returns_false_vec() {
        let preds = classify(&[], &[], &[vec![1.0, 2.0]], 3);
        assert_eq!(preds, vec![false]);
    }

    #[test]
    fn test_empty_test_returns_empty() {
        let train = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let preds = classify(&train, &labels, &[], 3);
        assert!(preds.is_empty());
    }

    #[test]
    fn test_depth_zero_returns_majority() {
        let train = vec![vec![0.0], vec![1.0], vec![0.5]];
        let labels = vec![false, false, true]; // majority = false
        let test = vec![vec![1.0]]; // would normally → true
        let preds = classify(&train, &labels, &test, 0);
        assert_eq!(preds, vec![false]); // depth=0 → leaf with majority
    }

    #[test]
    fn test_classify_d3_wrapper() {
        let train = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let test = vec![vec![0.0], vec![1.0]];
        let preds = classify_d3(&train, &labels, &test);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn test_all_same_label_returns_that_label() {
        let train = vec![vec![0.0], vec![0.5], vec![1.0]];
        let labels = vec![true, true, true];
        let test = vec![vec![0.0], vec![99.0]];
        let preds = classify(&train, &labels, &test, 5);
        assert_eq!(preds, vec![true, true]);
    }

    #[test]
    fn test_multifeature_split() {
        // Feature 0 is noise, feature 1 is signal.
        let train: Vec<Vec<f64>> = (0..20)
            .map(|i| vec![((i * 7) % 5) as f64, (i % 2) as f64])
            .collect();
        let labels: Vec<bool> = (0..20).map(|i| i % 2 == 1).collect();
        let test = vec![vec![0.0, 0.0], vec![0.0, 1.0]];
        let preds = classify(&train, &labels, &test, 4);
        assert_eq!(preds[0], false);
        assert_eq!(preds[1], true);
    }
}
