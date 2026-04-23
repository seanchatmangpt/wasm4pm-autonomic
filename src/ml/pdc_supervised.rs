//! Supervised ML classifier pipeline for PDC 2025 traces.
//!
//! Uses pseudo-labels derived from Petri net BFS conformance:
//! `true` = confirmed in-language trace (positive), `false` = pseudo-negative.
//!
//! All classifiers operate in a transductive setting: the same feature matrix is
//! used for both training and inference (train on all → predict on all).

use crate::ml::{
    decision_stump, decision_tree, gaussian_naive_bayes, gradient_boosting, knn, linear_regression,
    logistic_regression, naive_bayes, nearest_centroid, neural_network, perceptron,
};

// ---------------------------------------------------------------------------
// Result container
// ---------------------------------------------------------------------------

/// Predictions from every supervised classifier for a fixed set of traces.
///
/// Each field holds one `bool` per trace (same order as the `features` slice
/// passed to [`run_supervised`]).
#[derive(Debug, Default, Clone)]
pub struct SupervisedPredictions {
    pub knn: Vec<bool>,
    pub naive_bayes: Vec<bool>,
    pub decision_tree: Vec<bool>,
    pub logistic_regression: Vec<bool>,
    pub gaussian_nb: Vec<bool>,
    pub nearest_centroid: Vec<bool>,
    pub perceptron: Vec<bool>,
    pub neural_net: Vec<bool>,
    pub gradient_boosting: Vec<bool>,
    pub decision_stump: Vec<bool>,
    pub linear_classify: Vec<bool>,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Train all supervised classifiers on pseudo-labeled data and predict every trace.
///
/// `features` and `labels` must have the same length (one entry per trace).
/// The function operates transductively: every trace is both a training point
/// and a query point.
///
/// Returns [`SupervisedPredictions`] where each inner `Vec<bool>` has the same
/// length as `features`.  Empty `features` produces all-false predictions of
/// length zero.
pub fn run_supervised(features: &[Vec<f64>], labels: &[bool]) -> SupervisedPredictions {
    let n = features.len();

    // Guard: nothing to do if there are no traces or no features.
    if n == 0 {
        return SupervisedPredictions::default();
    }

    // Check whether any trace actually has features.  If all feature vectors
    // are empty every classifier that indexes into them would produce degenerate
    // results, so we short-circuit here too.
    let has_features = features.iter().any(|f| !f.is_empty());
    if !has_features {
        return SupervisedPredictions {
            knn: vec![false; n],
            naive_bayes: vec![false; n],
            decision_tree: vec![false; n],
            logistic_regression: vec![false; n],
            gaussian_nb: vec![false; n],
            nearest_centroid: vec![false; n],
            perceptron: vec![false; n],
            neural_net: vec![false; n],
            gradient_boosting: vec![false; n],
            decision_stump: vec![false; n],
            linear_classify: vec![false; n],
        };
    }

    // ── k-NN (k = 3, transductive: test == train) ─────────────────────────
    let knn = knn::classify_k3(features, labels, features);

    // ── Multinomial Naive Bayes ────────────────────────────────────────────
    let naive_bayes = naive_bayes::classify(features, labels, features);

    // ── Decision Tree (max_depth = 3) ──────────────────────────────────────
    let decision_tree = decision_tree::classify_d3(features, labels, features);

    // ── Logistic Regression (lr=0.01, epochs=1000) ─────────────────────────
    let logistic_regression = logistic_regression::classify_default(features, labels, features);

    // ── Gaussian Naive Bayes ───────────────────────────────────────────────
    let gaussian_nb = gaussian_naive_bayes::classify(features, labels, features);

    // ── Nearest Centroid ───────────────────────────────────────────────────
    let nearest_centroid = nearest_centroid::classify(features, labels, features);

    // ── Perceptron (100 epochs) ────────────────────────────────────────────
    let perceptron = perceptron::classify_default(features, labels, features);

    // ── Neural Network (hidden=4, lr=0.01, epochs=200) ────────────────────
    let neural_net = neural_network::classify_default(features, labels, features);

    // ── Gradient Boosting (50 estimators, lr=0.1) ─────────────────────────
    let gradient_boosting = gradient_boosting::classify_default(features, labels, features);

    // ── Decision Stump ────────────────────────────────────────────────────
    // decision_stump::classify calls predict which indexes x[stump.feature]
    // directly.  The empty-feature guard above ensures we never reach here with
    // empty rows, but we still check to be safe.
    let decision_stump = decision_stump::classify(features, labels, features);

    // ── Linear Regression thresholded at 0.5 ──────────────────────────────
    let linear_classify = linear_regression::classify_multiple(features, labels, features);

    SupervisedPredictions {
        knn,
        naive_bayes,
        decision_tree,
        logistic_regression,
        gaussian_nb,
        nearest_centroid,
        perceptron,
        neural_net,
        gradient_boosting,
        decision_stump,
        linear_classify,
    }
}

// ---------------------------------------------------------------------------
// Transfer (inductive) pipeline
// ---------------------------------------------------------------------------

/// Train supervised classifiers on labeled training data, predict on separate test features.
/// Both feature matrices must have the same column count (use a shared vocabulary).
pub fn run_supervised_transfer(
    train_features: &[Vec<f64>],
    train_labels: &[bool],
    test_features: &[Vec<f64>],
) -> SupervisedPredictions {
    let n_test = test_features.len();
    if train_features.is_empty() || n_test == 0 || !train_features.iter().any(|f| !f.is_empty()) {
        return SupervisedPredictions {
            knn: vec![false; n_test],
            naive_bayes: vec![false; n_test],
            decision_tree: vec![false; n_test],
            logistic_regression: vec![false; n_test],
            gaussian_nb: vec![false; n_test],
            nearest_centroid: vec![false; n_test],
            perceptron: vec![false; n_test],
            neural_net: vec![false; n_test],
            gradient_boosting: vec![false; n_test],
            decision_stump: vec![false; n_test],
            linear_classify: vec![false; n_test],
        };
    }
    SupervisedPredictions {
        knn: knn::classify_k3(train_features, train_labels, test_features),
        naive_bayes: naive_bayes::classify(train_features, train_labels, test_features),
        decision_tree: decision_tree::classify_d3(train_features, train_labels, test_features),
        logistic_regression: logistic_regression::classify_default(
            train_features,
            train_labels,
            test_features,
        ),
        gaussian_nb: gaussian_naive_bayes::classify(train_features, train_labels, test_features),
        nearest_centroid: nearest_centroid::classify(train_features, train_labels, test_features),
        perceptron: perceptron::classify_default(train_features, train_labels, test_features),
        neural_net: neural_network::classify_default(train_features, train_labels, test_features),
        gradient_boosting: gradient_boosting::classify_default(
            train_features,
            train_labels,
            test_features,
        ),
        decision_stump: decision_stump::classify(train_features, train_labels, test_features),
        linear_classify: linear_regression::classify_multiple(
            train_features,
            train_labels,
            test_features,
        ),
    }
}

// ---------------------------------------------------------------------------
// Inspection helper
// ---------------------------------------------------------------------------

/// Return a list of `(classifier_name, predictions)` pairs by cloning from `preds`.
///
/// The returned `Vec` always has exactly 11 entries, one per classifier, in the
/// same order as the fields of [`SupervisedPredictions`].
pub fn to_named_list(preds: &SupervisedPredictions) -> Vec<(&'static str, Vec<bool>)> {
    vec![
        ("knn", preds.knn.clone()),
        ("naive_bayes", preds.naive_bayes.clone()),
        ("decision_tree", preds.decision_tree.clone()),
        ("logistic_regression", preds.logistic_regression.clone()),
        ("gaussian_nb", preds.gaussian_nb.clone()),
        ("nearest_centroid", preds.nearest_centroid.clone()),
        ("perceptron", preds.perceptron.clone()),
        ("neural_net", preds.neural_net.clone()),
        ("gradient_boosting", preds.gradient_boosting.clone()),
        ("decision_stump", preds.decision_stump.clone()),
        ("linear_classify", preds.linear_classify.clone()),
    ]
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. Empty input ────────────────────────────────────────────────────────

    #[test]
    fn test_empty_features_returns_default() {
        let preds = run_supervised(&[], &[]);
        assert!(preds.knn.is_empty());
        assert!(preds.naive_bayes.is_empty());
        assert!(preds.decision_tree.is_empty());
        assert!(preds.logistic_regression.is_empty());
        assert!(preds.gaussian_nb.is_empty());
        assert!(preds.nearest_centroid.is_empty());
        assert!(preds.perceptron.is_empty());
        assert!(preds.neural_net.is_empty());
        assert!(preds.gradient_boosting.is_empty());
        assert!(preds.decision_stump.is_empty());
        assert!(preds.linear_classify.is_empty());
    }

    // ── 2. All-empty feature vectors (zero-dim) ───────────────────────────────

    #[test]
    fn test_all_empty_feature_vectors_returns_all_false() {
        let features = vec![vec![], vec![], vec![]];
        let labels = vec![true, false, true];
        let preds = run_supervised(&features, &labels);

        let named = to_named_list(&preds);
        for (name, v) in &named {
            assert_eq!(v.len(), 3, "classifier {name} must return 3 predictions");
            assert!(
                v.iter().all(|&b| !b),
                "classifier {name} must predict all-false for zero-dim features"
            );
        }
    }

    // ── 3. Basic structure — lengths match input ──────────────────────────────

    #[test]
    fn test_output_lengths_match_input() {
        // Simple 2-class, 2-feature dataset.
        let features = vec![
            vec![0.0, 0.0],
            vec![0.1, 0.0],
            vec![0.0, 0.1],
            vec![5.0, 5.0],
            vec![5.1, 5.0],
            vec![5.0, 5.1],
        ];
        let labels = vec![false, false, false, true, true, true];

        let preds = run_supervised(&features, &labels);
        let named = to_named_list(&preds);

        // Exactly 11 classifiers.
        assert_eq!(named.len(), 11);

        for (name, v) in &named {
            assert_eq!(
                v.len(),
                features.len(),
                "classifier {name} returned wrong number of predictions"
            );
        }
    }

    // ── 4. to_named_list returns 11 entries with correct names ────────────────

    #[test]
    fn test_to_named_list_names_and_count() {
        let preds = SupervisedPredictions::default();
        let named = to_named_list(&preds);
        assert_eq!(named.len(), 11);

        let expected_names = [
            "knn",
            "naive_bayes",
            "decision_tree",
            "logistic_regression",
            "gaussian_nb",
            "nearest_centroid",
            "perceptron",
            "neural_net",
            "gradient_boosting",
            "decision_stump",
            "linear_classify",
        ];
        for (i, (name, _)) in named.iter().enumerate() {
            assert_eq!(*name, expected_names[i]);
        }
    }

    // ── 5. run_supervised_transfer — basic lengths ────────────────────────────

    #[test]
    fn test_run_supervised_transfer_basic() {
        let train = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let labels = vec![true, false];
        let test = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.5, 0.5]];
        let preds = run_supervised_transfer(&train, &labels, &test);
        assert_eq!(preds.knn.len(), 3);
        assert_eq!(preds.neural_net.len(), 3);
    }

    // ── 6. run_supervised_transfer — empty guard ──────────────────────────────

    #[test]
    fn test_run_supervised_transfer_empty() {
        let preds = run_supervised_transfer(&[], &[], &[vec![1.0, 0.0]]);
        assert_eq!(preds.knn.len(), 1);
        assert!(!preds.knn[0]);
    }

    // ── 7. Clearly separable data — at least some classifiers predict correctly ─

    #[test]
    fn test_separable_data_some_classifiers_correct() {
        // Positive cluster at high values, negative at low values.
        let pos: Vec<Vec<f64>> = (0..10).map(|_| vec![10.0, 10.0]).collect();
        let neg: Vec<Vec<f64>> = (0..10).map(|_| vec![0.0, 0.0]).collect();
        let mut features = neg.clone();
        features.extend(pos);
        let mut labels = vec![false; 10];
        labels.extend(vec![true; 10]);

        let preds = run_supervised(&features, &labels);
        let named = to_named_list(&preds);

        let mut any_perfect = false;
        for (_name, v) in &named {
            let correct = v.iter().zip(labels.iter()).filter(|(p, l)| p == l).count();
            if correct == labels.len() {
                any_perfect = true;
            }
        }
        assert!(
            any_perfect,
            "at least one classifier should achieve perfect accuracy on linearly separable data"
        );
    }
}
