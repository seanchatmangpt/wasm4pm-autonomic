//! Synthetic-data ML training pipeline for PDC 2025.
//!
//! Generates positive/negative traces from a Petri net, trains multiple
//! classifiers on the synthetic data, and applies them to real contest traces.
//! This bypasses the 67.78% wall from approximate reference nets by using
//! known-label training data derived from the net's own language.

use crate::conformance::bitmask_replay::NetBitmask64;
use crate::models::{AttributeValue, EventLog};

use crate::conformance::trace_generator::{
    generate_negative_traces, generate_positive_traces, net_vocabulary,
};

// ── feature engineering ───────────────────────────────────────────────────────

/// Convert a raw activity sequence to a fixed-length feature vector.
///
/// Feature layout:
/// - `[0]` normalized trace length: `len / max_len`  (1.0 when `max_len == 0`)
/// - `[1]` unique activity ratio: `|distinct| / vocab_size` (0.0 when vocab empty)
/// - `[2..]` bag-of-words frequency: `count(activity_j) / max(len, 1)` for each
///           vocabulary position `j`
fn seq_to_features(activities: &[String], vocabulary: &[String], max_len: usize) -> Vec<f64> {
    let len = activities.len();

    // Feature 0: normalised length
    let norm_len = if max_len == 0 {
        1.0
    } else {
        len as f64 / max_len as f64
    };

    // Feature 1: unique activity ratio
    let unique_count = {
        let mut seen: Vec<&str> = Vec::with_capacity(len);
        for a in activities {
            if !seen.contains(&a.as_str()) {
                seen.push(a.as_str());
            }
        }
        seen.len()
    };
    let vocab_size = vocabulary.len();
    let unique_ratio = if vocab_size == 0 {
        0.0
    } else {
        unique_count as f64 / vocab_size as f64
    };

    // Features 2..: bag-of-words frequencies
    let denom = len.max(1) as f64;
    let bow: Vec<f64> = vocabulary
        .iter()
        .map(|word| {
            let count = activities.iter().filter(|a| *a == word).count();
            count as f64 / denom
        })
        .collect();

    let mut features = Vec::with_capacity(2 + bow.len());
    features.push(norm_len);
    features.push(unique_ratio);
    features.extend(bow);
    features
}

/// Build a feature matrix from a list of activity sequences.
///
/// `max_len` is derived internally as the maximum sequence length in `seqs`.
#[allow(dead_code)]
fn seqs_to_feature_matrix(seqs: &[Vec<String>], vocabulary: &[String]) -> Vec<Vec<f64>> {
    let max_len = seqs.iter().map(|s| s.len()).max().unwrap_or(0);
    seqs.iter()
        .map(|seq| seq_to_features(seq, vocabulary, max_len))
        .collect()
}

// ── result type ───────────────────────────────────────────────────────────────

/// Classification results from synthetic training applied to real traces.
///
/// Each field is a `Vec<bool>` of length equal to the number of real traces,
/// where `true` means the classifier predicts the trace is conforming (positive).
#[derive(Debug, Default, Clone)]
pub struct SyntheticResults {
    pub knn: Vec<bool>,
    pub naive_bayes: Vec<bool>,
    pub decision_tree: Vec<bool>,
    pub logistic_regression: Vec<bool>,
    pub gaussian_nb: Vec<bool>,
    pub nearest_centroid: Vec<bool>,
    pub neural_net: Vec<bool>,
    pub gradient_boosting: Vec<bool>,
    /// Majority vote across all eight classifiers, calibrated to `n_target`
    /// by selecting the `n_target` traces with the highest vote counts.
    pub ensemble: Vec<bool>,
}

// ── main pipeline ─────────────────────────────────────────────────────────────

/// Generate synthetic training data from a Petri net, train all classifiers,
/// and apply them to real contest traces.
///
/// # Arguments
/// - `net` — reference Petri net in bitmask form
/// - `real_traces` — activity sequences extracted from the contest log
/// - `n_synthetic` — number of positive and negative synthetic traces each
/// - `n_target` — number of positives to select in the ensemble (500 for PDC 2025)
///
/// # Returns
/// A [`SyntheticResults`] with per-classifier and ensemble predictions.
/// If the net is too restrictive to yield ≥ 10 positive traces, every field
/// is an all-`false` vector.
pub fn classify_with_synthetic(
    net: &NetBitmask64,
    real_traces: &[Vec<String>],
    n_synthetic: usize,
    n_target: usize,
) -> SyntheticResults {
    let n_real = real_traces.len();
    let default_result = SyntheticResults {
        knn: vec![false; n_real],
        naive_bayes: vec![false; n_real],
        decision_tree: vec![false; n_real],
        logistic_regression: vec![false; n_real],
        gaussian_nb: vec![false; n_real],
        nearest_centroid: vec![false; n_real],
        neural_net: vec![false; n_real],
        gradient_boosting: vec![false; n_real],
        ensemble: vec![false; n_real],
    };

    // Step 1: generate positive traces
    let positives = generate_positive_traces(net, n_synthetic, 200);
    if positives.len() < 10 {
        // Net too restrictive — cannot build a meaningful classifier
        return default_result;
    }

    // Step 2: net vocabulary
    let net_vocab = net_vocabulary(net);

    // Step 3: generate negative traces (seed = 42 for determinism)
    let negatives = generate_negative_traces(&positives, &net_vocab, 42);

    // Step 4: build COMBINED vocabulary = sorted union of net vocab + all real activities
    let mut combined_vocab: Vec<String> = net_vocab;
    for seq in real_traces {
        for act in seq {
            if !combined_vocab.contains(act) {
                combined_vocab.push(act.clone());
            }
        }
    }
    // Also include activities from synthetic traces not already covered
    for seq in positives.iter().chain(negatives.iter()) {
        for act in seq {
            if !combined_vocab.contains(act) {
                combined_vocab.push(act.clone());
            }
        }
    }
    combined_vocab.sort();

    // Step 5: compute max_len over ALL sequences (synthetic + real)
    let max_len = positives
        .iter()
        .chain(negatives.iter())
        .chain(real_traces.iter())
        .map(|s| s.len())
        .max()
        .unwrap_or(0);

    // Step 6: build synthetic feature matrix with labels
    let n_syn_total = positives.len() + negatives.len();
    let mut synthetic_labels: Vec<bool> = Vec::with_capacity(n_syn_total);
    let mut synthetic_seqs_owned: Vec<Vec<String>> = Vec::with_capacity(n_syn_total);

    for seq in &positives {
        synthetic_seqs_owned.push(seq.clone());
        synthetic_labels.push(true);
    }
    for seq in &negatives {
        synthetic_seqs_owned.push(seq.clone());
        synthetic_labels.push(false);
    }

    let synthetic_features: Vec<Vec<f64>> = synthetic_seqs_owned
        .iter()
        .map(|seq| seq_to_features(seq, &combined_vocab, max_len))
        .collect();

    // Step 7: build real feature matrix
    let real_features: Vec<Vec<f64>> = real_traces
        .iter()
        .map(|seq| seq_to_features(seq, &combined_vocab, max_len))
        .collect();

    if real_features.is_empty() {
        return default_result;
    }

    // Step 8: train each classifier on synthetic data, predict on real data
    let knn_preds =
        crate::ml::knn::classify_k3(&synthetic_features, &synthetic_labels, &real_features);

    let nb_preds =
        crate::ml::naive_bayes::classify(&synthetic_features, &synthetic_labels, &real_features);

    let dt_preds = crate::ml::decision_tree::classify_d3(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    let lr_preds = crate::ml::logistic_regression::classify_default(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    let gnb_preds = crate::ml::gaussian_naive_bayes::classify(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    let nc_preds = crate::ml::nearest_centroid::classify(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    let nn_preds = crate::ml::neural_network::classify_default(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    let gb_preds = crate::ml::gradient_boosting::classify_default(
        &synthetic_features,
        &synthetic_labels,
        &real_features,
    );

    // Step 9: ensemble — majority vote, calibrated to n_target
    let all_preds = [
        &knn_preds,
        &nb_preds,
        &dt_preds,
        &lr_preds,
        &gnb_preds,
        &nc_preds,
        &nn_preds,
        &gb_preds,
    ];

    // Count votes per trace
    let vote_counts: Vec<usize> = (0..n_real)
        .map(|i| all_preds.iter().filter(|preds| preds[i]).count())
        .collect();

    // Calibrate to n_target: take the n_target traces with most votes.
    // Ties are broken by trace index (lower index preferred — stable sort).
    let mut ranked: Vec<(usize, usize)> = vote_counts.iter().copied().enumerate().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1)); // descending by vote count

    let n_select = n_target.min(n_real);
    let mut ensemble = vec![false; n_real];
    for &(idx, _) in ranked.iter().take(n_select) {
        ensemble[idx] = true;
    }

    SyntheticResults {
        knn: knn_preds,
        naive_bayes: nb_preds,
        decision_tree: dt_preds,
        logistic_regression: lr_preds,
        gaussian_nb: gnb_preds,
        nearest_centroid: nc_preds,
        neural_net: nn_preds,
        gradient_boosting: gb_preds,
        ensemble,
    }
}

// ── log extraction ────────────────────────────────────────────────────────────

/// Extract activity sequences from an [`EventLog`].
///
/// For each trace, for each event, looks up the `concept:name` attribute and
/// returns it as a `String`.  Events without `concept:name` are skipped.
pub fn extract_sequences(log: &EventLog) -> Vec<Vec<String>> {
    log.traces
        .iter()
        .map(|trace| {
            trace
                .events
                .iter()
                .filter_map(|event| {
                    event.attributes.iter().find_map(|attr| {
                        if attr.key == "concept:name" {
                            if let AttributeValue::String(name) = &attr.value {
                                Some(name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                })
                .collect()
        })
        .collect()
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Attribute, AttributeValue, Event, EventLog, Trace};

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_event(name: &str) -> Event {
        Event {
            attributes: vec![Attribute {
                key: "concept:name".to_string(),
                value: AttributeValue::String(name.to_string()),
            }],
        }
    }

    fn make_trace(names: &[&str]) -> Trace {
        Trace {
            id: "t1".to_string(),
            events: names.iter().map(|n| make_event(n)).collect(),
            attributes: vec![],
        }
    }

    // ── test: extract_sequences ───────────────────────────────────────────────

    #[test]
    fn test_extract_sequences_basic() {
        let log = EventLog {
            traces: vec![
                make_trace(&["A", "B", "C"]),
                make_trace(&["X"]),
                make_trace(&[]),
            ],
            attributes: vec![],
        };
        let seqs = extract_sequences(&log);
        assert_eq!(seqs.len(), 3);
        assert_eq!(seqs[0], vec!["A", "B", "C"]);
        assert_eq!(seqs[1], vec!["X"]);
        assert!(seqs[2].is_empty());
    }

    #[test]
    fn test_extract_sequences_skips_non_string_concept_name() {
        // An event whose concept:name is an Int should be skipped.
        let log = EventLog {
            traces: vec![Trace {
                id: "t1".to_string(),
                events: vec![
                    Event {
                        attributes: vec![Attribute {
                            key: "concept:name".to_string(),
                            value: AttributeValue::Int(42),
                        }],
                    },
                    make_event("B"),
                ],
                attributes: vec![],
            }],
            attributes: vec![],
        };
        let seqs = extract_sequences(&log);
        assert_eq!(seqs[0], vec!["B"]);
    }

    // ── test: seq_to_features ─────────────────────────────────────────────────

    #[test]
    fn test_seq_to_features_shape_and_normalisation() {
        let vocab = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let seq = vec!["A".to_string(), "A".to_string(), "B".to_string()];
        let max_len = 6;
        let feats = seq_to_features(&seq, &vocab, max_len);

        // 2 header features + vocab_size BoW features
        assert_eq!(feats.len(), 5);

        // Feature 0: 3/6 = 0.5
        assert!(
            (feats[0] - 0.5).abs() < 1e-9,
            "norm length mismatch: {}",
            feats[0]
        );

        // Feature 1: unique ratio — {A, B} = 2 distinct / 3 vocab = 0.666...
        assert!(
            (feats[1] - 2.0 / 3.0).abs() < 1e-9,
            "unique ratio mismatch: {}",
            feats[1]
        );

        // BoW: A=2/3, B=1/3, C=0/3
        assert!(
            (feats[2] - 2.0 / 3.0).abs() < 1e-9,
            "A freq mismatch: {}",
            feats[2]
        );
        assert!(
            (feats[3] - 1.0 / 3.0).abs() < 1e-9,
            "B freq mismatch: {}",
            feats[3]
        );
        assert!(
            (feats[4] - 0.0).abs() < 1e-9,
            "C freq should be 0: {}",
            feats[4]
        );
    }

    #[test]
    fn test_seq_to_features_empty_sequence() {
        let vocab = vec!["A".to_string()];
        let feats = seq_to_features(&[], &vocab, 10);
        // norm_len = 0/10 = 0.0; unique_ratio = 0/1 = 0.0; BoW A = 0/1 = 0.0
        assert_eq!(feats.len(), 3);
        assert!((feats[0] - 0.0).abs() < 1e-9);
        assert!((feats[1] - 0.0).abs() < 1e-9);
        assert!((feats[2] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_seq_to_features_max_len_zero() {
        let vocab = vec!["A".to_string()];
        // When max_len=0 the norm_len should be 1.0 (not NaN/inf)
        let feats = seq_to_features(&[], &vocab, 0);
        assert!(
            (feats[0] - 1.0).abs() < 1e-9,
            "expected 1.0 when max_len=0, got {}",
            feats[0]
        );
    }

    // ── test: seqs_to_feature_matrix ──────────────────────────────────────────

    #[test]
    fn test_seqs_to_feature_matrix_consistent_width() {
        let vocab = vec!["A".to_string(), "B".to_string()];
        let seqs = vec![
            vec!["A".to_string()],
            vec!["B".to_string(), "B".to_string()],
            vec![],
        ];
        let matrix = seqs_to_feature_matrix(&seqs, &vocab);
        assert_eq!(matrix.len(), 3);
        // All rows must have the same width: 2 (header) + 2 (vocab)
        for row in &matrix {
            assert_eq!(row.len(), 4, "row width mismatch: {:?}", row);
        }
    }

    // ── test: classify_with_synthetic (stub path) ─────────────────────────────

    /// When trace_generator is absent (cfg not feature="trace-generator"),
    /// generate_positive_traces returns vec![], which is < 10 positives.
    /// The pipeline must return the all-false default result without panicking.
    #[test]
    fn test_classify_with_synthetic_stub_returns_default() {
        use crate::conformance::bitmask_replay::NetBitmask64;
        use crate::models::petri_net::PetriNet;

        // Build a minimal valid PetriNet (1 place, 0 transitions, ≤64 places).
        let mut net = PetriNet::default();
        net.places.push(crate::models::petri_net::Place {
            id: "p0".to_string(),
        });
        let bm = NetBitmask64::from_petri_net(&net);

        let real_traces = vec![vec!["A".to_string()], vec!["B".to_string()]];
        let result = classify_with_synthetic(&bm, &real_traces, 50, 1);

        // With stub, positives.len() == 0 < 10 → all-false default
        assert_eq!(result.knn, vec![false, false]);
        assert_eq!(result.ensemble, vec![false, false]);
    }

    // ── test: ensemble calibration ────────────────────────────────────────────

    #[test]
    fn test_ensemble_vote_count_calibration() {
        // Directly test the vote-counting + calibration logic.
        // We simulate vote_counts for 5 traces and want n_target = 2.
        let vote_counts: Vec<usize> = vec![3, 7, 1, 5, 7];
        // Ranked descending: indices 1(7), 4(7), 3(5), 0(3), 2(1)
        // Top 2 → indices 1 and 4
        let n_real = 5;
        let n_target = 2;
        let mut ranked: Vec<(usize, usize)> = vote_counts.iter().copied().enumerate().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let mut ensemble = vec![false; n_real];
        for &(idx, _) in ranked.iter().take(n_target) {
            ensemble[idx] = true;
        }
        assert_eq!(ensemble[1], true, "index 1 should be selected (7 votes)");
        assert_eq!(ensemble[4], true, "index 4 should be selected (7 votes)");
        assert_eq!(ensemble[0], false);
        assert_eq!(ensemble[2], false);
        assert_eq!(ensemble[3], false);
        assert_eq!(
            ensemble.iter().filter(|&&v| v).count(),
            2,
            "exactly 2 selected"
        );
    }
}
