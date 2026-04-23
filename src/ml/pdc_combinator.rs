//! Full combinatorial wiring for PDC 2025.
//!
//! Pools supervised, unsupervised, and synthetic classifier outputs with all
//! conformance score signals and runs exhaustive/greedy ensemble search,
//! optionally guided by the 6-bit process-parameter configuration encoded in
//! the log filename.

use crate::conformance::bitmask_replay::{classify_exact, replay_log, NetBitmask64};
use crate::ml::hdc;
use crate::ml::pdc_ensemble::{
    best_bool_score_pair, calibrate_to_target, combinatorial_ensemble, full_combinatorial,
    greedy_ensemble, score, vote_fractions,
};
use crate::ml::pdc_features::{
    build_vocabulary, extract_log_features, extract_log_features_with_vocab, pseudo_labels,
};
use crate::ml::pdc_supervised::{
    run_supervised, run_supervised_transfer, to_named_list as sup_named,
};
use crate::ml::pdc_unsupervised::{run_unsupervised, to_named_list as unsup_named};
use crate::ml::rank_fusion::{bool_to_score, borda_count, reciprocal_rank_fusion};
use crate::ml::stacking::stack_ensemble;
use crate::ml::synthetic_trainer::classify_with_synthetic;
use crate::ml::weighted_vote::{auto_weighted_vote, precision_weighted_vote};
use crate::models::{AttributeValue, EventLog};

// ── Parameter config ──────────────────────────────────────────────────────────

/// 6-bit PDC 2025 process-parameter configuration decoded from a log filename
/// like `pdc2025_010101.xes`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pdc2025Config {
    pub dependent_tasks: bool,
    pub loops: u8, // 0 = none, 1 = simple, 2 = complex
    pub or_constructs: bool,
    pub routing: bool,
    pub optional_tasks: bool,
    pub duplicate_tasks: bool,
}

impl Pdc2025Config {
    /// Parse from filename.  Returns `None` if the 6-digit suffix is absent or
    /// contains non-digit characters.
    pub fn from_log_name(name: &str) -> Option<Self> {
        let base = name.trim_end_matches(".xes");
        let suffix = base.rsplit('_').next()?;
        if suffix.len() != 6 {
            return None;
        }
        let d: Vec<u8> = suffix
            .chars()
            .map(|c| c.to_digit(10).map(|x| x as u8))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            dependent_tasks: d[0] != 0,
            loops: d[1],
            or_constructs: d[2] != 0,
            routing: d[3] != 0,
            optional_tasks: d[4] != 0,
            duplicate_tasks: d[5] != 0,
        })
    }

    /// Composite complexity score in `[0.0, 1.0]`.
    pub fn complexity_score(&self) -> f64 {
        let mut s = 0.0_f64;
        if self.dependent_tasks {
            s += 0.20;
        }
        s += match self.loops {
            0 => 0.0,
            1 => 0.15,
            _ => 0.25,
        };
        if self.or_constructs {
            s += 0.20;
        }
        if self.routing {
            s += 0.10;
        }
        if self.optional_tasks {
            s += 0.15;
        }
        if self.duplicate_tasks {
            s += 0.10;
        }
        s
    }

    pub fn has_complex_loops(&self) -> bool {
        self.loops == 2
    }
}

// ── Output type ───────────────────────────────────────────────────────────────

/// One solution from the combinatorial search, ranked by anchor score.
#[derive(Debug, Clone)]
pub struct CombinatorResult {
    /// Binary predictions (exactly `n_target` positives).
    pub predictions: Vec<bool>,
    /// Score on anchor pseudo-labels (recall + precision penalty).
    pub score: f64,
    /// Human-readable label identifying which sub-strategy produced this.
    pub strategy_name: String,
    /// Number of classifier outputs pooled into this result.
    pub n_classifiers: usize,
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Run the full combinatorial wiring.
///
/// 1. Extract 7+vocab features (fitness, in_language, norm_len, norm_unique,
///    is_perfect, missing_norm, remaining_norm, BoW).
/// 2. Pool supervised (11) + unsupervised (6) + synthetic (1 ensemble) + HDC (1) + baseline
///    classify_exact → up to 20 bool predictions.
/// 3. Build 4 score signals (fitness, is_perfect, 1-missing_norm, 1-remaining_norm).
/// 4. Run `full_combinatorial`, `best_bool_score_pair`, `combinatorial_ensemble`,
///    and optional parameter-aware routing.
/// 5. Return results sorted by score descending.
///
/// * `log_name` — optional filename used to decode the 6-bit parameter config.
/// * `train_labeled` — optional `(training_log, labels)` for supervised transfer learning.
///   When `Some`, trains classifiers on real labels (typically 40 labeled traces from the
///   `_11` file) instead of 1000 pseudo-labeled test traces — ~24× faster per benchmark.
pub fn run_combinator(
    log: &EventLog,
    net: &NetBitmask64,
    n_target: usize,
    log_name: Option<&str>,
    train_labeled: Option<(&EventLog, &[bool])>,
) -> Vec<CombinatorResult> {
    // ── 1. Feature extraction + conformance signals ───────────────────────────
    let (features, in_lang, fitness) = if let Some((train_log, _)) = train_labeled {
        // Build shared vocabulary so train and test features live in the same space.
        let mut vocab = build_vocabulary(log);
        for w in build_vocabulary(train_log) {
            if !vocab.contains(&w) {
                vocab.push(w);
            }
        }
        vocab.sort();
        let max_len = log
            .traces
            .iter()
            .chain(train_log.traces.iter())
            .map(|t| t.events.len())
            .max()
            .unwrap_or(1);
        extract_log_features_with_vocab(log, net, &vocab, max_len)
    } else {
        extract_log_features(log, net)
    };

    let replay = replay_log(net, log);
    let is_perfect_scores: Vec<f64> = replay
        .iter()
        .map(|r| if r.is_perfect() { 1.0 } else { 0.0 })
        .collect();
    let missing_norm: Vec<f64> = replay
        .iter()
        .map(|r| {
            let d = (r.consumed + r.missing) as f64;
            if d == 0.0 {
                0.0
            } else {
                r.missing as f64 / d
            }
        })
        .collect();
    let remaining_norm: Vec<f64> = replay
        .iter()
        .map(|r| {
            let d = (r.produced + r.remaining) as f64;
            if d == 0.0 {
                0.0
            } else {
                r.remaining as f64 / d
            }
        })
        .collect();

    let anchor = &in_lang;

    // ── 2. Parameter-aware config ─────────────────────────────────────────────
    let config = log_name.and_then(Pdc2025Config::from_log_name);
    let complexity = config.map(|c| c.complexity_score()).unwrap_or(0.5);
    let n_synthetic = if complexity < 0.3 {
        500
    } else if complexity < 0.6 {
        1_000
    } else {
        2_000
    };

    // ── 3. Build classifier pool ──────────────────────────────────────────────
    // Pseudo-labels for unsupervised classifiers (always needed)
    let pseudo_opt = pseudo_labels(&in_lang, &fitness);
    let pseudo_bool: Vec<bool> = pseudo_opt.iter().map(|p| p.unwrap_or(false)).collect();

    // Use real labels when available (24× faster: 40 train traces vs 1000 pseudo-labeled).
    let sup = if let Some((train_log, train_lbls)) = train_labeled {
        let vocab = {
            let mut v = build_vocabulary(log);
            for w in build_vocabulary(train_log) {
                if !v.contains(&w) {
                    v.push(w);
                }
            }
            v.sort();
            v
        };
        let max_len = log
            .traces
            .iter()
            .chain(train_log.traces.iter())
            .map(|t| t.events.len())
            .max()
            .unwrap_or(1);
        let (train_feats, _, _) = extract_log_features_with_vocab(train_log, net, &vocab, max_len);
        run_supervised_transfer(&train_feats, train_lbls, &features)
    } else {
        run_supervised(&features, &pseudo_bool)
    };
    let unsup = run_unsupervised(&features, &pseudo_opt, &fitness, n_target);

    let act_seqs: Vec<Vec<String>> = log
        .traces
        .iter()
        .map(|t| {
            t.events
                .iter()
                .filter_map(|e| {
                    e.attributes
                        .iter()
                        .find(|a| a.key == "concept:name")
                        .and_then(|a| {
                            if let AttributeValue::String(s) = &a.value {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                })
                .collect()
        })
        .collect();
    let synth = classify_with_synthetic(net, &act_seqs, n_synthetic, n_target);

    // HDC transductive: fit on test sequences, classify same sequences (+1)
    let hdc_clf = hdc::fit(&act_seqs);
    let hdc_preds = hdc::classify(&hdc_clf, &act_seqs, n_target);

    // ── 4. Collect all bool predictions ──────────────────────────────────────
    let mut bool_preds: Vec<Vec<bool>> = Vec::new();
    bool_preds.push(classify_exact(net, log, n_target)); // baseline
    for (_, v) in sup_named(&sup) {
        bool_preds.push(v);
    } // +11
    for (_, v) in unsup_named(&unsup) {
        bool_preds.push(v);
    } // +6
    bool_preds.push(synth.ensemble.clone()); // +1
    bool_preds.push(hdc_preds); //              +1  → total ≤20

    // ── 5. Score signals (higher = more positive, bounded [0,1]) ─────────────
    let score_signals: Vec<Vec<f64>> = vec![
        fitness.clone(),
        is_perfect_scores.clone(),
        missing_norm.iter().map(|v| 1.0 - v).collect(),
        remaining_norm.iter().map(|v| 1.0 - v).collect(),
    ];

    // ── 4b. Meta-ensemble derived bool predictions ────────────────────────────
    let awv_pred = auto_weighted_vote(&bool_preds, anchor, n_target);
    let pwv_pred = precision_weighted_vote(&bool_preds, anchor, n_target);
    let stack_pred = stack_ensemble(&bool_preds, anchor, n_target);

    // ── 5b. Rank-fusion bool predictions → extended score signals ─────────────
    let higher_is_better = vec![true; score_signals.len()];
    let borda_pred = borda_count(&score_signals, &higher_is_better, n_target);
    let rrf_pred = reciprocal_rank_fusion(&score_signals, &higher_is_better, n_target);

    let mut extended_score_signals = score_signals.clone();
    extended_score_signals.push(bool_to_score(&borda_pred));
    extended_score_signals.push(bool_to_score(&rrf_pred));
    extended_score_signals.push(bool_to_score(&awv_pred));
    extended_score_signals.push(bool_to_score(&pwv_pred));
    extended_score_signals.push(bool_to_score(&stack_pred));

    let mut extended_bool_preds = bool_preds.clone();
    extended_bool_preds.push(borda_pred.clone());
    extended_bool_preds.push(rrf_pred.clone());
    extended_bool_preds.push(awv_pred.clone());
    extended_bool_preds.push(pwv_pred.clone());
    extended_bool_preds.push(stack_pred.clone());

    // ── 6. Combinatorial search ───────────────────────────────────────────────
    let mut results: Vec<CombinatorResult> = Vec::new();

    // 6a. Full combinatorial (bool + score → exploits all 2^(k+m) combos)
    {
        let k = bool_preds.len();
        let preds = full_combinatorial(&bool_preds, &score_signals, anchor, n_target);
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: "full_combinatorial".into(),
            n_classifiers: k,
        });
    }

    // 6b. Best bool+score pair (fastest; O(k·m·n log n))
    {
        let preds = best_bool_score_pair(&bool_preds, &score_signals, anchor, n_target);
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: "best_pair".into(),
            n_classifiers: 2,
        });
    }

    // 6c. Combinatorial ensemble on bool predictions only
    {
        let k = bool_preds.len();
        let preds = combinatorial_ensemble(&bool_preds, anchor, n_target);
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: "combo_bool_only".into(),
            n_classifiers: k,
        });
    }

    // 6d. Parameter-aware routing (only when filename config is available)
    if let Some(cfg) = config {
        let preds = if cfg.has_complex_loops() {
            // Complex-loop nets: synthetic classifier captures loop structure best
            let fracs = vote_fractions(&bool_preds[bool_preds.len().saturating_sub(4)..]);
            calibrate_to_target(&synth.ensemble, &fracs, n_target)
        } else {
            // Simple / no-loop nets: fitness + is_perfect are reliable
            best_bool_score_pair(
                &bool_preds[..3.min(bool_preds.len())],
                &score_signals[..2],
                anchor,
                n_target,
            )
        };
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: format!(
                "param_aware(loops={},complexity={:.2})",
                cfg.loops, complexity
            ),
            n_classifiers: 3,
        });
    }

    // 6e. full_combinatorial with extended score signals (original bool pool, 9 signals)
    {
        let k = bool_preds.len();
        let m = extended_score_signals.len();
        let preds = full_combinatorial(&bool_preds, &extended_score_signals, anchor, n_target);
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: format!("full_combo_ext(k={k},m={m})"),
            n_classifiers: k + m,
        });
    }

    // 6f. greedy ensemble on extended bool pool (25 preds)
    {
        let k = extended_bool_preds.len();
        let preds = greedy_ensemble(&extended_bool_preds, anchor, n_target);
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: format!("greedy_ext(k={k})"),
            n_classifiers: k,
        });
    }

    // 6g. best_pair on extended pools
    {
        let k = extended_bool_preds.len();
        let preds = best_bool_score_pair(
            &extended_bool_preds,
            &extended_score_signals,
            anchor,
            n_target,
        );
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: format!("best_pair_ext(k={k})"),
            n_classifiers: 2,
        });
    }

    // 6h. named individual derived predictions (already computed, zero extra cost)
    for (name, preds) in [
        ("borda_fusion", borda_pred),
        ("rrf_fusion", rrf_pred),
        ("auto_weighted_vote", awv_pred),
        ("prec_weighted_vote", pwv_pred),
        ("stack_ensemble", stack_pred),
    ] {
        let s = score(&preds, anchor, n_target);
        results.push(CombinatorResult {
            predictions: preds,
            score: s,
            strategy_name: name.into(),
            n_classifiers: bool_preds.len(),
        });
    }

    // Sort best first
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        // "010101" → d = [0,1,0,1,0,1]
        let c = Pdc2025Config::from_log_name("pdc2025_010101.xes").unwrap();
        assert!(!c.dependent_tasks); // d[0]=0
        assert_eq!(c.loops, 1); // d[1]=1
        assert!(!c.or_constructs); // d[2]=0
        assert!(c.routing); // d[3]=1
        assert!(!c.optional_tasks); // d[4]=0
        assert!(c.duplicate_tasks); // d[5]=1

        // "121111" → d = [1,2,1,1,1,1]
        let c2 = Pdc2025Config::from_log_name("pdc2025_121111.xes").unwrap();
        assert!(c2.dependent_tasks);
        assert_eq!(c2.loops, 2);
        assert!(c2.has_complex_loops());
        assert!(c2.or_constructs);
        assert!(c2.routing);
        assert!(c2.optional_tasks);
        assert!(c2.duplicate_tasks);
    }

    #[test]
    fn test_config_complexity_bounds() {
        let min = Pdc2025Config::from_log_name("pdc2025_000000.xes").unwrap();
        let max = Pdc2025Config::from_log_name("pdc2025_121111.xes").unwrap();
        assert!(min.complexity_score() < max.complexity_score());
        assert!(min.complexity_score() >= 0.0);
        assert!(max.complexity_score() <= 1.0);
    }

    #[test]
    fn test_config_none_on_bad_name() {
        assert!(Pdc2025Config::from_log_name("unknown.xes").is_none());
        assert!(Pdc2025Config::from_log_name("pdc2025_1234567.xes").is_none());
    }
}
