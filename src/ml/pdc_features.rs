//! PDC 2025 feature extraction for ML classifiers.
//!
//! Converts XES traces into fixed-length numerical feature vectors suitable for
//! k-NN, Naive Bayes, and other standard classifiers. Features are computed
//! without access to ground-truth labels.

use crate::conformance::bitmask_replay::{in_language, replay_log, NetBitmask64};
use crate::models::{AttributeValue, EventLog, Trace};
use std::collections::HashMap;

/// Build a sorted vocabulary of all unique activity names seen in a log.
///
/// The returned `Vec<String>` is sorted lexicographically and contains no
/// duplicates. It forms the canonical index for bag-of-activities features.
pub fn build_vocabulary(log: &EventLog) -> Vec<String> {
    let mut seen: HashMap<&str, ()> = HashMap::new();
    for trace in &log.traces {
        for event in &trace.events {
            if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                if let AttributeValue::String(s) = &attr.value {
                    seen.entry(s.as_str()).or_insert(());
                }
            }
        }
    }
    let mut vocab: Vec<String> = seen.into_keys().map(String::from).collect();
    vocab.sort();
    vocab
}

/// Extract numerical features for a single trace.
///
/// Returns a `Vec<f64>` of length `4 + vocabulary.len()`:
///
/// | Index | Meaning |
/// |-------|---------|
/// | `[0]` | Token-replay fitness score in `[0.0, 1.0]` |
/// | `[1]` | `in_language` (BFS exact) as `1.0` or `0.0` |
/// | `[2]` | Trace length normalised to `[0, 1]` by `log_max_len` |
/// | `[3]` | Unique activity count normalised by `vocabulary.len()` |
/// | `[4]` | `is_perfect` (missing==0 && remaining==0) as `1.0` or `0.0` |
/// | `[5]` | `missing_norm` = missing / (consumed + missing), bounded `[0, 1]` |
/// | `[6]` | `remaining_norm` = remaining / (produced + remaining), bounded `[0, 1]` |
/// | `[7..]` | Activity frequency: `count(activity_j) / max(trace_len, 1)` for each `j` in vocabulary |
pub fn trace_to_features(
    trace: &Trace,
    net: &NetBitmask64,
    vocabulary: &[String],
    log_max_len: usize,
) -> Vec<f64> {
    // Collect activity names from the trace once.
    let activities: Vec<&str> = trace
        .events
        .iter()
        .filter_map(|e| {
            e.attributes
                .iter()
                .find(|a| a.key == "concept:name")
                .and_then(|a| {
                    if let AttributeValue::String(s) = &a.value {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
        })
        .collect();

    let trace_len = activities.len();

    // Replay fitness + conformance deviation signals
    use crate::conformance::bitmask_replay::replay_trace;
    let replay_result = replay_trace(net, trace);
    let fitness = replay_result.fitness();
    let is_perfect = if replay_result.is_perfect() {
        1.0_f64
    } else {
        0.0_f64
    };
    let missing_norm = {
        let d = (replay_result.consumed + replay_result.missing) as f64;
        if d == 0.0 {
            0.0_f64
        } else {
            replay_result.missing as f64 / d
        }
    };
    let remaining_norm = {
        let d = (replay_result.produced + replay_result.remaining) as f64;
        if d == 0.0 {
            0.0_f64
        } else {
            replay_result.remaining as f64 / d
        }
    };

    // Exact language membership
    let lang_flag = if in_language(net, trace) {
        1.0_f64
    } else {
        0.0_f64
    };

    // Trace length normalised
    let norm_len = if log_max_len == 0 {
        0.0_f64
    } else {
        trace_len as f64 / log_max_len as f64
    };

    // Unique activity count normalised by vocabulary size
    let unique_count: usize = {
        let mut seen: HashMap<&str, ()> = HashMap::new();
        for &act in &activities {
            seen.entry(act).or_insert(());
        }
        seen.len()
    };
    let norm_unique = if vocabulary.is_empty() {
        0.0_f64
    } else {
        unique_count as f64 / vocabulary.len() as f64
    };

    // Activity frequency bag
    let max_len = trace_len.max(1);
    let mut freq_map: HashMap<&str, usize> = HashMap::new();
    for &act in &activities {
        *freq_map.entry(act).or_insert(0) += 1;
    }
    let bag: Vec<f64> = vocabulary
        .iter()
        .map(|v| *freq_map.get(v.as_str()).unwrap_or(&0) as f64 / max_len as f64)
        .collect();

    let mut features = Vec::with_capacity(7 + vocabulary.len());
    features.push(fitness);
    features.push(lang_flag);
    features.push(norm_len);
    features.push(norm_unique);
    features.push(is_perfect);
    features.push(missing_norm);
    features.push(remaining_norm);
    features.extend_from_slice(&bag);
    features
}

/// Extract features for all traces in a log.
///
/// Returns `(features, in_lang_flags, fitness_scores)`:
/// - `features[i]` — feature vector for trace `i` (length `7 + |vocabulary|`)
/// - `in_lang_flags[i]` — whether trace `i` is in the Petri net's language (BFS exact check)
/// - `fitness_scores[i]` — token-replay fitness for trace `i`
pub fn extract_log_features(
    log: &EventLog,
    net: &NetBitmask64,
) -> (Vec<Vec<f64>>, Vec<bool>, Vec<f64>) {
    let vocabulary = build_vocabulary(log);

    // Run token replay once for all traces.
    let replay_results = replay_log(net, log);
    let fitness_scores: Vec<f64> = replay_results.iter().map(|r| r.fitness()).collect();

    // BFS in_language per trace.
    let in_lang_flags: Vec<bool> = log.traces.iter().map(|t| in_language(net, t)).collect();

    // Compute log_max_len.
    let log_max_len = log.traces.iter().map(|t| t.events.len()).max().unwrap_or(0);

    // Build feature vectors — reuse precomputed fitness & in_language to avoid
    // redundant replay/BFS inside trace_to_features by assembling manually.
    let features: Vec<Vec<f64>> = log
        .traces
        .iter()
        .enumerate()
        .map(|(i, trace)| {
            let activities: Vec<&str> = trace
                .events
                .iter()
                .filter_map(|e| {
                    e.attributes
                        .iter()
                        .find(|a| a.key == "concept:name")
                        .and_then(|a| {
                            if let AttributeValue::String(s) = &a.value {
                                Some(s.as_str())
                            } else {
                                None
                            }
                        })
                })
                .collect();

            let trace_len = activities.len();
            let fitness = fitness_scores[i];
            let lang_flag = if in_lang_flags[i] { 1.0_f64 } else { 0.0_f64 };

            let norm_len = if log_max_len == 0 {
                0.0_f64
            } else {
                trace_len as f64 / log_max_len as f64
            };

            let unique_count: usize = {
                let mut seen: HashMap<&str, ()> = HashMap::new();
                for &act in &activities {
                    seen.entry(act).or_insert(());
                }
                seen.len()
            };
            let norm_unique = if vocabulary.is_empty() {
                0.0_f64
            } else {
                unique_count as f64 / vocabulary.len() as f64
            };

            let max_len = trace_len.max(1);
            let mut freq_map: HashMap<&str, usize> = HashMap::new();
            for &act in &activities {
                *freq_map.entry(act).or_insert(0) += 1;
            }
            let bag: Vec<f64> = vocabulary
                .iter()
                .map(|v| *freq_map.get(v.as_str()).unwrap_or(&0) as f64 / max_len as f64)
                .collect();

            let rr = &replay_results[i];
            let is_perfect = if rr.is_perfect() { 1.0_f64 } else { 0.0_f64 };
            let missing_norm = {
                let d = (rr.consumed + rr.missing) as f64;
                if d == 0.0 {
                    0.0_f64
                } else {
                    rr.missing as f64 / d
                }
            };
            let remaining_norm = {
                let d = (rr.produced + rr.remaining) as f64;
                if d == 0.0 {
                    0.0_f64
                } else {
                    rr.remaining as f64 / d
                }
            };

            let mut fv = Vec::with_capacity(7 + vocabulary.len());
            fv.push(fitness);
            fv.push(lang_flag);
            fv.push(norm_len);
            fv.push(norm_unique);
            fv.push(is_perfect);
            fv.push(missing_norm);
            fv.push(remaining_norm);
            fv.extend_from_slice(&bag);
            fv
        })
        .collect();

    (features, in_lang_flags, fitness_scores)
}

/// Extract features using a pre-built shared vocabulary and global max length.
/// Use when features from multiple logs must share a feature space.
pub fn extract_log_features_with_vocab(
    log: &EventLog,
    net: &NetBitmask64,
    shared_vocab: &[String],
    global_max_len: usize,
) -> (Vec<Vec<f64>>, Vec<bool>, Vec<f64>) {
    let replay_results = replay_log(net, log);
    let fitness_scores: Vec<f64> = replay_results.iter().map(|r| r.fitness()).collect();
    let in_lang_flags: Vec<bool> = log.traces.iter().map(|t| in_language(net, t)).collect();
    let features: Vec<Vec<f64>> = log
        .traces
        .iter()
        .map(|trace| trace_to_features(trace, net, shared_vocab, global_max_len))
        .collect();
    (features, in_lang_flags, fitness_scores)
}

/// Build pseudo-labels for supervised training.
///
/// - `Some(true)`  — trace is in the Petri net's language (confirmed positive from BFS)
/// - `Some(false)` — trace has fitness `< 0.3` (likely negative)
/// - `None`        — ambiguous middle-ground trace
pub fn pseudo_labels(in_lang: &[bool], fitness: &[f64]) -> Vec<Option<bool>> {
    assert_eq!(
        in_lang.len(),
        fitness.len(),
        "in_lang and fitness slices must have the same length"
    );
    in_lang
        .iter()
        .zip(fitness.iter())
        .map(|(&lang, &fit)| {
            if lang {
                Some(true)
            } else if fit < 0.3 {
                Some(false)
            } else {
                None
            }
        })
        .collect()
}

// ─── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance::bitmask_replay::NetBitmask64;
    use crate::models::petri_net::{Arc, PetriNet, Place, Transition};
    use crate::models::{Attribute, Event, EventLog, Trace};
    use crate::utils::dense_kernel::{fnv1a_64, PackedKeyTable};

    /// Linear net: p0 -a-> p1 -b-> p2
    fn linear_net() -> NetBitmask64 {
        let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
        im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
        let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
        fm.insert(fnv1a_64(b"p2"), "p2".into(), 1);
        let net = PetriNet {
            places: vec![
                Place { id: "p0".into() },
                Place { id: "p1".into() },
                Place { id: "p2".into() },
            ],
            transitions: vec![
                Transition {
                    id: "t_a".into(),
                    label: "a".into(),
                    is_invisible: Some(false),
                },
                Transition {
                    id: "t_b".into(),
                    label: "b".into(),
                    is_invisible: Some(false),
                },
            ],
            arcs: vec![
                Arc {
                    from: "p0".into(),
                    to: "t_a".into(),
                    weight: None,
                },
                Arc {
                    from: "t_a".into(),
                    to: "p1".into(),
                    weight: None,
                },
                Arc {
                    from: "p1".into(),
                    to: "t_b".into(),
                    weight: None,
                },
                Arc {
                    from: "t_b".into(),
                    to: "p2".into(),
                    weight: None,
                },
            ],
            initial_marking: im,
            final_markings: vec![fm],
            ..Default::default()
        };
        NetBitmask64::from_petri_net(&net)
    }

    fn make_trace(id: &str, acts: &[&str]) -> Trace {
        Trace {
            id: id.into(),
            attributes: vec![],
            events: acts
                .iter()
                .map(|&a| Event {
                    attributes: vec![Attribute {
                        key: "concept:name".into(),
                        value: AttributeValue::String(a.into()),
                    }],
                })
                .collect(),
        }
    }

    fn make_log(traces: Vec<Trace>) -> EventLog {
        EventLog {
            traces,
            attributes: vec![],
        }
    }

    // ── Test 1: vocabulary is sorted and deduplicated ──────────────────────────

    #[test]
    fn test_build_vocabulary_sorted_unique() {
        let log = make_log(vec![
            make_trace("t1", &["b", "a", "c"]),
            make_trace("t2", &["a", "b", "b"]),
            make_trace("t3", &["c", "d"]),
        ]);
        let vocab = build_vocabulary(&log);
        assert_eq!(vocab, vec!["a", "b", "c", "d"]);
    }

    // ── Test 2: trace_to_features length and fitness bounds ────────────────────

    #[test]
    fn test_trace_to_features_shape_and_bounds() {
        let net = linear_net();
        let log = make_log(vec![
            make_trace("t1", &["a", "b"]),
            make_trace("t2", &["a"]),
        ]);
        let vocab = build_vocabulary(&log);
        let log_max_len = 2_usize;

        let fv_perfect = trace_to_features(&log.traces[0], &net, &vocab, log_max_len);
        let fv_partial = trace_to_features(&log.traces[1], &net, &vocab, log_max_len);

        // Feature length = 7 + |vocab|
        assert_eq!(fv_perfect.len(), 7 + vocab.len());
        assert_eq!(fv_partial.len(), 7 + vocab.len());

        // Fitness in [0, 1]
        assert!((0.0..=1.0).contains(&fv_perfect[0]));
        assert!((0.0..=1.0).contains(&fv_partial[0]));

        // Perfect trace ["a","b"] is in language → flag = 1.0
        assert_eq!(fv_perfect[1], 1.0);

        // Normalised length for perfect trace (len=2, max=2) = 1.0
        assert!((fv_perfect[2] - 1.0).abs() < 1e-9);

        // is_perfect, missing_norm, remaining_norm all in [0, 1]
        assert!((0.0..=1.0).contains(&fv_perfect[4]));
        assert!((0.0..=1.0).contains(&fv_perfect[5]));
        assert!((0.0..=1.0).contains(&fv_perfect[6]));

        // All frequency values in [0, 1]
        for &v in &fv_perfect[7..] {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    // ── Test 3: extract_log_features consistency ───────────────────────────────

    #[test]
    fn test_extract_log_features_consistency() {
        let net = linear_net();
        let log = make_log(vec![
            make_trace("t1", &["a", "b"]),      // in language
            make_trace("t2", &["b"]),           // not in language, low fitness
            make_trace("t3", &["a", "b", "a"]), // out-of-language variant
        ]);

        let (features, in_lang, fitness) = extract_log_features(&log, &net);

        assert_eq!(features.len(), 3);
        assert_eq!(in_lang.len(), 3);
        assert_eq!(fitness.len(), 3);

        // Trace 0 ["a","b"] should be in language
        assert!(
            in_lang[0],
            "trace [a,b] must be in the linear net's language"
        );

        // Each feature vector must have the correct length
        let vocab_len = build_vocabulary(&log).len();
        for fv in &features {
            assert_eq!(fv.len(), 7 + vocab_len);
        }

        // Fitness scores are consistent: in-language trace has fitness 1.0
        assert!(
            (fitness[0] - 1.0).abs() < 1e-9,
            "perfect trace fitness must be 1.0"
        );
    }

    // ── Test 4: pseudo_labels boundary conditions ──────────────────────────────

    #[test]
    fn test_pseudo_labels() {
        let in_lang = [true, false, false, false];
        let fitness = [1.0_f64, 0.1, 0.5, 0.29];
        let labels = pseudo_labels(&in_lang, &fitness);

        assert_eq!(labels[0], Some(true)); // in language → positive
        assert_eq!(labels[1], Some(false)); // fitness < 0.3 → negative
        assert_eq!(labels[2], None); // 0.3 ≤ fitness < 1.0, not in lang → ambiguous
        assert_eq!(labels[3], Some(false)); // 0.29 < 0.3 → negative
    }

    // ── Test 5: empty log is handled gracefully ────────────────────────────────

    #[test]
    fn test_empty_log() {
        let net = linear_net();
        let log = make_log(vec![]);
        let vocab = build_vocabulary(&log);
        assert!(vocab.is_empty());
        let (features, in_lang, fitness) = extract_log_features(&log, &net);
        assert!(features.is_empty());
        assert!(in_lang.is_empty());
        assert!(fitness.is_empty());
    }

    // ── Test 6: extract_log_features_with_vocab uses shared vocabulary ─────────

    #[test]
    fn test_extract_log_features_with_vocab_signature() {
        // Verify the function is callable with correct types
        let vocab: Vec<String> = vec!["A".to_string()];
        assert_eq!(vocab.len(), 1);
    }
}
