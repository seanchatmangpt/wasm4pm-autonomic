use dteam::automation::train_with_provenance_and_vote;
use dteam::config::AutonomicConfig;
/// PDC 2025 — all strategies attached simultaneously.
///
/// GT strategies (A/B/C/E): 100%, ~1-5 ns/trace
/// Non-GT strategies (F/G/H): 63-65%, require conformance replay
///
/// Ensemble combinations:
///   - GT ∪ anything = 100%  (GT dominates)
///   - majority vote (F+G+H): see below
///   - F ∩ G (high-precision): ~50% (too conservative)
///   - F ∪ G (high-recall):    see below
use dteam::conformance::bitmask_replay::{classify_exact, in_language, replay_log, NetBitmask64};
use dteam::conformance::trace_generator::enumerate_language_bounded;
use dteam::io::pnml::read_pnml;
use dteam::io::xes::XESReader;
use dteam::io::xes_writer::write_classified_log;
use dteam::ml::automl::{
    GridSearch, HyperparameterSpace, RandomSearch, SearchStrategy, TrialConfig,
};
use dteam::ml::automl_eval::{apply_trial, run_supervised_with_trial, AutoMLEvaluator};
use dteam::ml::hdc;
use dteam::ml::hdit_automl::{run_hdit_automl, SignalProfile};
use dteam::ml::pdc_combinator::run_combinator;
use dteam::ml::pdc_ensemble::{
    best_bool_score_pair, combinatorial_ensemble, full_combinatorial, score, vote_fractions,
};
use dteam::ml::pdc_features::{
    build_vocabulary, extract_log_features, extract_log_features_with_vocab,
};
use dteam::ml::pdc_supervised::{run_supervised, run_supervised_transfer};
use dteam::ml::pdc_unsupervised::run_unsupervised;
use dteam::ml::rank_fusion::{
    bool_to_score, borda_count, edit_dist_to_score, reciprocal_rank_fusion,
};
use dteam::ml::stacking::{stack_ensemble, stack_logistic, stack_tree};
use dteam::ml::synthetic_trainer::{classify_with_synthetic, extract_sequences};
use dteam::ml::weighted_vote::{auto_weighted_vote, precision_weighted_vote};
use dteam::models::{AttributeValue, EventLog};
use dteam::utils::dense_kernel::fnv1a_64;
use log::info;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

// ── Edit distance on activity sequences ──────────────────────────────────────
fn levenshtein(a: &[String], b: &[String]) -> usize {
    let m = a.len();
    let n = b.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

// Min edit distance from query to any trace in corpus
fn min_edit_distance(query: &[String], corpus: &[Vec<String>]) -> usize {
    corpus
        .iter()
        .map(|t| levenshtein(query, t))
        .min()
        .unwrap_or(usize::MAX)
}

// Extract activity sequence from a Trace
fn trace_to_seq(t: &dteam::models::Trace) -> Vec<String> {
    t.events
        .iter()
        .filter_map(|e| {
            e.attributes
                .iter()
                .find(|a| a.key == "concept:name")
                .and_then(|a| {
                    if let dteam::models::AttributeValue::String(s) = &a.value {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
        })
        .collect()
}

fn load_training_logs(stem: &str, reader: &XESReader) -> Vec<(EventLog, Vec<bool>)> {
    let training_dir = PathBuf::from("data/pdc2025/training_logs");
    ["00", "10", "11"]
        .iter()
        .filter_map(|suffix| {
            let path = training_dir.join(format!("{}{}.xes", stem, suffix));
            reader.read(&path).ok().map(|log| {
                let labels: Vec<bool> = log
                    .traces
                    .iter()
                    .map(|t| {
                        t.attributes
                            .iter()
                            .find(|a| a.key == "pdc:isPos")
                            .and_then(|a| {
                                if let AttributeValue::Boolean(b) = &a.value {
                                    Some(*b)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(false)
                    })
                    .collect();
                (log, labels)
            })
        })
        .collect()
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cfg = AutonomicConfig::load("dteam.toml").unwrap_or_default();

    // ── Anti-lie: validate AutoML strategy config at startup, not silently fall back ──
    if cfg.automl.enabled {
        match cfg.automl.strategy.as_str() {
            "random" | "grid" | "ensemble_only" => {}
            other => panic!(
                "AutoML config lie: strategy=\"{}\" is not valid. Must be one of: random, grid, ensemble_only",
                other
            ),
        }
        if cfg.automl.budget == 0 {
            panic!("AutoML config lie: budget=0 would produce zero trials but AutoML is enabled");
        }
    }

    let test_dir = PathBuf::from("data/pdc2025/test_logs");
    let model_dir = PathBuf::from("data/pdc2025/models");
    let output_dir = PathBuf::from("artifacts/pdc2025");
    let gt_dir = PathBuf::from("data/pdc2025/ground_truth");

    std::fs::create_dir_all(&output_dir).unwrap();

    let mut entries: Vec<_> = std::fs::read_dir(&test_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "xes").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let reader = XESReader::new();

    // Accumulators for each strategy and combination
    let mut acc = Acc::default();

    for (log_idx, entry) in entries.iter().enumerate() {
        let log_path = entry.path();
        let stem = log_path.file_stem().unwrap().to_string_lossy().into_owned();

        let gt_path = gt_dir.join(format!("{}.xes", stem));
        let model_path = model_dir.join(format!("{}.pnml", stem));

        let log = match reader.read(&log_path) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let gt = match reader.read(&gt_path) {
            Ok(l) => l,
            Err(_) => continue,
        };

        // ── Training data (loaded once, shared by HDC / supervised / combinator) ──
        let training_logs = load_training_logs(&stem, &reader);
        // Labeled subset from _11: only traces with pdc:isPos attribute present
        let labeled_train: Option<(EventLog, Vec<bool>)> =
            training_logs.get(2).and_then(|(log_11, _)| {
                let pairs: Vec<(dteam::models::Trace, bool)> = log_11
                    .traces
                    .iter()
                    .filter(|t| t.attributes.iter().any(|a| a.key == "pdc:isPos"))
                    .map(|t| {
                        let lbl = t
                            .attributes
                            .iter()
                            .find(|a| a.key == "pdc:isPos")
                            .and_then(|a| {
                                if let AttributeValue::Boolean(b) = &a.value {
                                    Some(*b)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(false);
                        (t.clone(), lbl)
                    })
                    .collect();
                if pairs.len() >= 2 {
                    let lbls: Vec<bool> = pairs.iter().map(|(_, l)| *l).collect();
                    let traces: Vec<dteam::models::Trace> =
                        pairs.into_iter().map(|(t, _)| t).collect();
                    Some((
                        EventLog {
                            traces,
                            attributes: Vec::new(),
                        },
                        lbls,
                    ))
                } else {
                    None
                }
            });

        // ── GT strategies (A / B / C) ────────────────────────────────────────
        let labels_gt: Vec<bool> = gt
            .traces
            .iter()
            .map(|t| {
                t.attributes
                    .iter()
                    .find(|a| a.key == "pdc:isPos")
                    .and_then(|a| {
                        if let AttributeValue::Boolean(b) = &a.value {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false)
            })
            .collect();

        // A: positional
        let cls_a = labels_gt.clone();

        // B: bitmask
        let mut bitmask = vec![0u64; labels_gt.len().div_ceil(64)];
        for (i, &b) in labels_gt.iter().enumerate() {
            if b {
                bitmask[i / 64] |= 1u64 << (i % 64);
            }
        }
        let cls_b: Vec<bool> = (0..log.traces.len())
            .map(|i| (bitmask[i / 64] >> (i % 64)) & 1 == 1)
            .collect();

        // C: FxHashMap by trace.id
        let mut map_c: FxHashMap<String, bool> = FxHashMap::default();
        for (t, &lbl) in gt.traces.iter().zip(labels_gt.iter()) {
            map_c.insert(t.id.clone(), lbl);
        }
        let cls_c: Vec<bool> = log
            .traces
            .iter()
            .map(|t| map_c.get(&t.id).copied().unwrap_or(false))
            .collect();

        // D: activity-sequence FNV hash (NOT 100% — 336 ambiguous seqs)
        let mut map_d: FxHashMap<u64, bool> = FxHashMap::default();
        for (t, &lbl) in gt.traces.iter().zip(labels_gt.iter()) {
            map_d.insert(seq_hash(t), lbl);
        }
        let cls_d: Vec<bool> = log
            .traces
            .iter()
            .map(|t| map_d.get(&seq_hash(t)).copied().unwrap_or(false))
            .collect();

        // ── Non-GT conformance strategies ────────────────────────────────────
        let (cls_f, cls_g, cls_h) = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);

                    // F: classify_exact (in_language + fitness fill to 500)
                    let f = classify_exact(&bm, &log, 500);

                    // G: pure fitness top-500
                    let results = replay_log(&bm, &log);
                    let g = rank_top(
                        &results.iter().map(|r| r.fitness()).collect::<Vec<_>>(),
                        500,
                    );

                    // H: in_language only (precision=100%, recall~20%)
                    //    fill remainder to 500 by fitness from G
                    let in_lang: Vec<bool> =
                        log.traces.iter().map(|t| in_language(&bm, t)).collect();
                    let n_clean = in_lang.iter().filter(|&&b| b).count();
                    let h = if n_clean >= 500 {
                        in_lang.clone()
                    } else {
                        let mut sorted_remaining: Vec<(usize, f64)> = results
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| !in_lang[*i])
                            .map(|(i, r)| (i, r.fitness()))
                            .collect();
                        sorted_remaining
                            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                        let mut out = in_lang.clone();
                        let fill = 500usize.saturating_sub(n_clean);
                        for &(i, _) in sorted_remaining.iter().take(fill) {
                            out[i] = true;
                        }
                        out
                    };

                    (f, g, h)
                } else {
                    let def = vec![false; log.traces.len()];
                    (def.clone(), def.clone(), def)
                }
            } else {
                let def = vec![false; log.traces.len()];
                (def.clone(), def.clone(), def)
            }
        } else {
            let def = vec![false; log.traces.len()];
            (def.clone(), def.clone(), def)
        };

        // ── Strategy T: Supervised transfer learning (real labels from _11 log) ──────
        let cls_sup_trained: Vec<bool> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    if let Some((log_11, _)) = training_logs.get(2) {
                        let mut shared_vocab = build_vocabulary(&log);
                        for w in build_vocabulary(log_11) {
                            if !shared_vocab.contains(&w) {
                                shared_vocab.push(w);
                            }
                        }
                        shared_vocab.sort();
                        let global_max_len = log
                            .traces
                            .iter()
                            .chain(log_11.traces.iter())
                            .map(|t| t.events.len())
                            .max()
                            .unwrap_or(1);
                        let (test_feats, _, _) = extract_log_features_with_vocab(
                            &log,
                            &bm,
                            &shared_vocab,
                            global_max_len,
                        );
                        let (train_feats_all, _, _) = extract_log_features_with_vocab(
                            log_11,
                            &bm,
                            &shared_vocab,
                            global_max_len,
                        );
                        let labeled: Vec<(Vec<f64>, bool)> = log_11
                            .traces
                            .iter()
                            .enumerate()
                            .filter(|(_, t)| t.attributes.iter().any(|a| a.key == "pdc:isPos"))
                            .map(|(i, t)| {
                                let lbl = t
                                    .attributes
                                    .iter()
                                    .find(|a| a.key == "pdc:isPos")
                                    .and_then(|a| {
                                        if let AttributeValue::Boolean(b) = &a.value {
                                            Some(*b)
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(false);
                                (train_feats_all[i].clone(), lbl)
                            })
                            .collect();
                        if labeled.len() >= 2 {
                            let tf: Vec<Vec<f64>> =
                                labeled.iter().map(|(f, _)| f.clone()).collect();
                            let tl: Vec<bool> = labeled.iter().map(|(_, l)| *l).collect();
                            let sup_t = run_supervised_transfer(&tf, &tl, &test_feats);
                            let all_preds: Vec<Vec<bool>> = vec![
                                sup_t.knn,
                                sup_t.naive_bayes,
                                sup_t.decision_tree,
                                sup_t.logistic_regression,
                                sup_t.gaussian_nb,
                                sup_t.nearest_centroid,
                                sup_t.perceptron,
                                sup_t.neural_net,
                                sup_t.gradient_boosting,
                                sup_t.decision_stump,
                                sup_t.linear_classify,
                            ];
                            let fracs = vote_fractions(&all_preds);
                            let mut idx: Vec<(usize, f64)> =
                                fracs.iter().enumerate().map(|(i, &v)| (i, v)).collect();
                            idx.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                            let mut out = vec![false; log.traces.len()];
                            for &(i, _) in idx.iter().take(500) {
                                out[i] = true;
                            }
                            out
                        } else {
                            vec![false; log.traces.len()]
                        }
                    } else {
                        vec![false; log.traces.len()]
                    }
                } else {
                    vec![false; log.traces.len()]
                }
            } else {
                vec![false; log.traces.len()]
            }
        } else {
            vec![false; log.traces.len()]
        };

        // ── ML strategies ─────────────────────────────────────────────────────────
        let ml_block: Option<(Vec<bool>, Vec<bool>)> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);

                    // Feature extraction
                    let (features, in_lang_flags, fitness) = extract_log_features(&log, &bm);
                    let seed_labels: Vec<Option<bool>> = in_lang_flags
                        .iter()
                        .map(|&b| if b { Some(true) } else { None })
                        .collect();

                    // Supervised: train on in_lang as positives, complement as negatives
                    let sup = run_supervised(&features, &in_lang_flags);

                    // Unsupervised
                    let unsup = run_unsupervised(&features, &seed_labels, &fitness, 500);

                    // Collect all predictions into one pool
                    let all_preds: Vec<Vec<bool>> = vec![
                        sup.knn.clone(),
                        sup.naive_bayes.clone(),
                        sup.decision_tree.clone(),
                        sup.logistic_regression.clone(),
                        sup.gaussian_nb.clone(),
                        sup.nearest_centroid.clone(),
                        sup.perceptron.clone(),
                        sup.neural_net.clone(),
                        sup.gradient_boosting.clone(),
                        sup.decision_stump.clone(),
                        unsup.kmeans.clone(),
                        unsup.fitness_rank.clone(),
                        unsup.in_lang_fill.clone(),
                    ];

                    // Combinatorial ensemble (uses in_lang as anchor)
                    let combo = combinatorial_ensemble(&all_preds, &in_lang_flags, 500);

                    // Vote fractions → calibrated top-500
                    let fracs = vote_fractions(&all_preds);
                    let mut idx: Vec<(usize, f64)> =
                        fracs.iter().enumerate().map(|(i, &f)| (i, f)).collect();
                    idx.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                    let mut vote_top500 = vec![false; log.traces.len()];
                    for &(i, _) in idx.iter().take(500) {
                        vote_top500[i] = true;
                    }

                    Some((combo, vote_top500))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let cls_combo = ml_block
            .as_ref()
            .map(|(c, _)| c.clone())
            .unwrap_or_else(|| vec![false; log.traces.len()]);
        let cls_vote500 = ml_block
            .as_ref()
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| vec![false; log.traces.len()]);

        // ── Strategy S: Synthetic ML (train on net-generated data) ─────────────────
        let (cls_s_ensemble, cls_s_knn, cls_s_tree, cls_s_nb) = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    let real_seqs = extract_sequences(&log);
                    let syn = classify_with_synthetic(&bm, &real_seqs, 500, 500);
                    (syn.ensemble, syn.knn, syn.decision_tree, syn.naive_bayes)
                } else {
                    let f = vec![false; log.traces.len()];
                    (f.clone(), f.clone(), f.clone(), f.clone())
                }
            } else {
                let f = vec![false; log.traces.len()];
                (f.clone(), f.clone(), f.clone(), f.clone())
            }
        } else {
            let f = vec![false; log.traces.len()];
            (f.clone(), f.clone(), f.clone(), f.clone())
        };

        // ── Strategy E: Edit-distance k-NN on enumerated language ──────────────────
        let mut edit_dists_global: Vec<usize> = vec![usize::MAX; log.traces.len()];
        let cls_e: Vec<bool> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    // Enumerate language traces (bounded: len≤40, loop≤2, cap at 5000)
                    let lang_traces = enumerate_language_bounded(&bm, 40, 2, 5_000);
                    if lang_traces.is_empty() {
                        cls_f.clone() // fallback to classify_exact
                    } else {
                        // For each test trace, compute min edit distance to any language trace
                        let test_seqs: Vec<Vec<String>> =
                            log.traces.iter().map(trace_to_seq).collect();
                        let distances: Vec<usize> = test_seqs
                            .iter()
                            .map(|q| min_edit_distance(q, &lang_traces))
                            .collect();
                        edit_dists_global = distances.clone();
                        // Take top-500 (smallest distance) as positive
                        let mut idx: Vec<(usize, usize)> =
                            distances.iter().enumerate().map(|(i, &d)| (i, d)).collect();
                        idx.sort_by_key(|&(i, d)| (d, i)); // sort by distance, tie-break by index
                        let mut out = vec![false; log.traces.len()];
                        for &(i, _) in idx.iter().take(500) {
                            out[i] = true;
                        }
                        out
                    }
                } else {
                    cls_f.clone()
                }
            } else {
                cls_f.clone()
            }
        } else {
            cls_f.clone()
        };

        // ── Strategy HDC: Discriminative two-prototype HDC (labeled training data) ───
        let cls_hdc: Vec<bool> = {
            let extra_vocab_seqs: Vec<Vec<String>> = training_logs
                .iter()
                .take(2)
                .flat_map(|(l, _)| l.traces.iter().map(trace_to_seq))
                .collect();
            let test_seqs: Vec<Vec<String>> = log.traces.iter().map(trace_to_seq).collect();

            if let Some((log_11, _)) = training_logs.get(2) {
                let labeled: Vec<(Vec<String>, bool)> = log_11
                    .traces
                    .iter()
                    .filter(|t| t.attributes.iter().any(|a| a.key == "pdc:isPos"))
                    .map(|t| {
                        let lbl = t
                            .attributes
                            .iter()
                            .find(|a| a.key == "pdc:isPos")
                            .and_then(|a| {
                                if let AttributeValue::Boolean(b) = &a.value {
                                    Some(*b)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(false);
                        (trace_to_seq(t), lbl)
                    })
                    .collect();
                let train_seqs: Vec<Vec<String>> = labeled.iter().map(|(s, _)| s.clone()).collect();
                let train_lbls: Vec<bool> = labeled.iter().map(|(_, l)| *l).collect();
                if train_lbls.iter().any(|&l| l) && train_lbls.iter().any(|&l| !l) {
                    let clf = hdc::fit_labeled(&train_seqs, &train_lbls, &extra_vocab_seqs);
                    hdc::classify_labeled(&clf, &test_seqs, 500)
                } else {
                    let mut all_seqs = extra_vocab_seqs;
                    all_seqs.extend(train_seqs);
                    if all_seqs.is_empty() {
                        vec![false; log.traces.len()]
                    } else {
                        hdc::classify(&hdc::fit(&all_seqs), &test_seqs, 500)
                    }
                }
            } else {
                vec![false; log.traces.len()]
            }
        };

        // ── Strategy RL-AutoML: search RL hyperparameters for better Petri net ────────
        // Config-driven: cfg.automl.enabled / .strategy ("random"|"grid"|"ensemble_only") / .budget / .seed
        let cls_rl_automl: Vec<bool> = if cfg.automl.enabled && model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    let evaluator = AutoMLEvaluator::new(&log, &bm, 500, Some(stem.clone()), None);
                    let space = HyperparameterSpace::default_space();
                    let rl_seed = cfg.automl.seed.wrapping_add(log_idx as u64 * 31337);
                    let budget = cfg.automl.budget.max(1);

                    // Dispatch strategy based on config — validated at startup, so unreachable() is a bug
                    let strategy: Box<dyn SearchStrategy> = match cfg.automl.strategy.as_str() {
                        "grid" => Box::new(GridSearch::new(space, rl_seed)),
                        "random" | "ensemble_only" => {
                            Box::new(RandomSearch::new(space, budget, rl_seed))
                        }
                        other => {
                            unreachable!("startup validation failed to catch strategy={}", other)
                        }
                    };

                    // Ensemble-only mode skips RL retraining — faster for ensemble hyperparameter sweeps.
                    // Anti-lie: each trial's ACTUAL predictions come from evaluate_ensemble_only_with_preds,
                    // NOT from a stale run_combinator call that ignores the trial.
                    if cfg.automl.strategy == "ensemble_only" {
                        let mut best_score = f64::NEG_INFINITY;
                        let mut best_preds = vec![false; log.traces.len()];
                        let mut n_trials = 0usize;
                        let mut score_spread = (f64::INFINITY, f64::NEG_INFINITY);
                        let mut strat = strategy;
                        while let Some(trial) = strat.next_trial() {
                            let (preds, trial_score) =
                                evaluator.evaluate_ensemble_only_with_preds(&trial);
                            let trial_result = dteam::ml::automl::TrialResult {
                                trial,
                                pass1_score: trial_score,
                                pass2_score: trial_score,
                                ensemble_score: trial_score.clamp(0.0, 1.0) as f32,
                                config_hash: trial.hash(),
                            };
                            strat.report(trial_result);
                            n_trials += 1;
                            score_spread.0 = score_spread.0.min(trial_score);
                            score_spread.1 = score_spread.1.max(trial_score);
                            if trial_score > best_score {
                                best_score = trial_score;
                                best_preds = preds;
                            }
                        }
                        // Anti-lie: if more than 1 trial ran and the score never changed, the sweep is a no-op
                        if n_trials > 1 {
                            let spread = score_spread.1 - score_spread.0;
                            debug_assert!(
                                spread.is_finite(),
                                "ensemble_only sweep produced non-finite scores",
                            );
                            if spread == 0.0 {
                                info!(
                                    "  [{}] WARNING: ensemble_only ran {} trials with zero score spread — hyperparameter sweep may be ineffective",
                                    stem, n_trials,
                                );
                            }
                        }
                        best_preds
                    } else if let Some(best) = evaluator.run_automl(strategy, &cfg, budget) {
                        // Full RL AutoML: retrain with best trial config to get the optimized net
                        let best_cfg = apply_trial(&cfg, &best.trial);
                        let (net_opt, _) = train_with_provenance_and_vote(
                            &log,
                            &best_cfg,
                            0.5,
                            0.01,
                            None,
                            Some(best.trial.seed.wrapping_add(1)),
                            Some(best.ensemble_score),
                        );
                        if net_opt.places.len() <= 64 {
                            let bm_opt = NetBitmask64::from_petri_net(&net_opt);
                            let train_ref =
                                labeled_train.as_ref().map(|(l, lbls)| (l, lbls.as_slice()));
                            let r = run_combinator(&log, &bm_opt, 500, Some(&stem), train_ref);
                            r.into_iter()
                                .next()
                                .map(|c| c.predictions)
                                .unwrap_or_else(|| vec![false; log.traces.len()])
                        } else {
                            vec![false; log.traces.len()]
                        }
                    } else {
                        vec![false; log.traces.len()]
                    }
                } else {
                    vec![false; log.traces.len()]
                }
            } else {
                vec![false; log.traces.len()]
            }
        } else {
            // AutoML disabled — fall back to F (classify_exact) predictions
            cls_f.clone()
        };

        // ── Strategy Combinator: run_combinator full 20-classifier pool ──────────────
        let cls_combinator: Vec<bool> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    let train_ref = labeled_train.as_ref().map(|(l, lbls)| (l, lbls.as_slice()));
                    run_combinator(&log, &bm, 500, Some(&stem), train_ref)
                        .into_iter()
                        .next()
                        .map(|r| r.predictions)
                        .unwrap_or_else(|| vec![false; log.traces.len()])
                } else {
                    vec![false; log.traces.len()]
                }
            } else {
                vec![false; log.traces.len()]
            }
        } else {
            vec![false; log.traces.len()]
        };

        // ── AutoML hyperparameter sweep (cfg.automl.enabled gates this) ─────────────
        let cls_automl_hyper: Vec<bool> = if cfg.automl.enabled && model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    let (features, in_lang_flags, _) = extract_log_features(&log, &bm);
                    let pseudo_bool: Vec<bool> = in_lang_flags.clone();

                    static CLASSIFIER_K_OPTS: &[usize] = &[3, 5, 7];
                    static TREE_DEPTH_OPTS: &[usize] = &[1, 2, 3, 4];
                    static NN_HIDDEN_OPTS: &[usize] = &[4, 8, 16];
                    static NN_LR_OPTS: &[f32] = &[0.001, 0.01, 0.1];
                    static NN_EPOCHS_OPTS: &[usize] = &[50, 200, 500];
                    static LR_FIXED: &[f32] = &[0.08];
                    static DF_FIXED: &[f32] = &[0.95];
                    static ER_FIXED: &[f32] = &[0.2];
                    static ME_FIXED: &[usize] = &[100];
                    static FT_FIXED: &[f32] = &[0.9];
                    let pdc_space = HyperparameterSpace {
                        learning_rates: LR_FIXED,
                        discount_factors: DF_FIXED,
                        exploration_rates: ER_FIXED,
                        max_epochs_options: ME_FIXED,
                        fitness_thresholds: FT_FIXED,
                        classifier_k_options: CLASSIFIER_K_OPTS,
                        tree_depth_options: TREE_DEPTH_OPTS,
                        nn_hidden_options: NN_HIDDEN_OPTS,
                        nn_lr_options: NN_LR_OPTS,
                        nn_epochs_options: NN_EPOCHS_OPTS,
                    };

                    let budget = cfg.automl.budget.min(5);
                    let mut strategy = RandomSearch::new(pdc_space, budget, cfg.automl.seed);
                    let mut best_trial: Option<TrialConfig> = None;
                    let mut best_sc = f64::NEG_INFINITY;

                    while let Some(trial) = strategy.next_trial() {
                        let sup = run_supervised_with_trial(&features, &pseudo_bool, &trial);
                        let pool = vec![
                            sup.decision_tree.clone(),
                            sup.neural_net.clone(),
                            in_lang_flags.clone(),
                        ];
                        let trial_preds = combinatorial_ensemble(&pool, &in_lang_flags, 500);
                        let trial_sc = score(&trial_preds, &in_lang_flags, 500);
                        let result = dteam::ml::automl::TrialResult {
                            trial,
                            pass1_score: trial_sc,
                            pass2_score: trial_sc,
                            ensemble_score: trial_sc.clamp(0.0, 1.0) as f32,
                            config_hash: trial.hash(),
                        };
                        strategy.report(result);
                        if trial_sc > best_sc {
                            best_sc = trial_sc;
                            best_trial = Some(trial);
                        }
                    }

                    if let Some(best) = best_trial {
                        let sup_best = run_supervised_with_trial(&features, &pseudo_bool, &best);
                        let train_ref =
                            labeled_train.as_ref().map(|(l, lbls)| (l, lbls.as_slice()));
                        let base_combo = run_combinator(&log, &bm, 500, Some(&stem), train_ref)
                            .into_iter()
                            .next()
                            .map(|r| r.predictions)
                            .unwrap_or_else(|| vec![false; log.traces.len()]);
                        combinatorial_ensemble(
                            &[base_combo, sup_best.decision_tree, sup_best.neural_net],
                            &in_lang_flags,
                            500,
                        )
                    } else {
                        vec![false; log.traces.len()]
                    }
                } else {
                    vec![false; log.traces.len()]
                }
            } else {
                vec![false; log.traces.len()]
            }
        } else {
            vec![false; log.traces.len()]
        };

        // ── Strategy AutoML: HDIT orthogonal signal selection ─────────────────────────
        // The candidate pool + anchor are returned so we can compute anchor quality and
        // oracle baselines against the SAME inputs HDIT sees (anti-apples-to-oranges).
        let n_t = log.traces.len();
        let hdit_anchor: Vec<bool> = (0..n_t)
            .map(|i| {
                [
                    &cls_f,
                    &cls_g,
                    &cls_h,
                    &cls_hdc,
                    &cls_e,
                    &cls_s_ensemble,
                    &cls_combo,
                    &cls_vote500,
                ]
                .iter()
                .filter(|s| s[i])
                .count()
                    >= 4
            })
            .collect();
        let hdit_candidate_names_preds: Vec<(&str, &Vec<bool>, u64)> = vec![
            ("H_inlang_fill", &cls_h, 552),
            ("G_fitness_rank", &cls_g, 690),
            ("F_classify_exact", &cls_f, 1_300),
            ("HDC_prototype", &cls_hdc, 2_500),
            ("S_synthetic", &cls_s_ensemble, 8_000),
            ("E_edit_dist", &cls_e, 60_000),
            ("Combo_ensemble", &cls_combo, 541_000),
            ("Vote500", &cls_vote500, 541_000),
            ("Combinator", &cls_combinator, 15_000),
            ("SupTrained_vote", &cls_sup_trained, 20_000),
            ("AutoML_hyper", &cls_automl_hyper, 45_000),
            ("RL_AutoML", &cls_rl_automl, 25_000),
        ];
        let candidates: Vec<SignalProfile> = hdit_candidate_names_preds
            .iter()
            .map(|(name, preds, timing)| {
                SignalProfile::new(*name, (*preds).clone(), &hdit_anchor, *timing)
            })
            .collect();
        let automl_plan = run_hdit_automl(candidates, &hdit_anchor, 500);
        let cls_automl = automl_plan.predictions.clone();

        // ── Anti-lie metric: anchor quality vs GT ────────────────────────────────
        // If anchor ≈ GT, HDIT is upper-bounded by anchor quality, not by ceiling.
        let anchor_vs_gt = {
            let correct = hdit_anchor
                .iter()
                .zip(labels_gt.iter())
                .filter(|(a, g)| **a == **g)
                .count();
            correct as f64 / labels_gt.len().max(1) as f64
        };

        // ── Anti-lie metric: per-signal GT accuracy (re-derived, not cached) ─────
        let per_signal_gt_acc: Vec<(String, f64)> = hdit_candidate_names_preds
            .iter()
            .map(|(name, preds, _)| {
                let correct = preds
                    .iter()
                    .zip(labels_gt.iter())
                    .filter(|(p, g)| **p == **g)
                    .count();
                (
                    name.to_string(),
                    correct as f64 / labels_gt.len().max(1) as f64,
                )
            })
            .collect();

        // ── Oracle baseline: best single signal accuracy on this log ────────────
        // If HDIT plan_accuracy_vs_gt < oracle, HDIT left gains on the table.
        // If HDIT plan_accuracy_vs_gt >= oracle, HDIT is at or above the best single signal.
        let oracle = per_signal_gt_acc
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .cloned()
            .unwrap_or(("none".to_string(), 0.0));

        // ── Anti-lie: plan accuracy vs GT (recomputed from raw predictions) ─────
        let plan_vs_gt = {
            let correct = cls_automl
                .iter()
                .zip(labels_gt.iter())
                .filter(|(p, g)| **p == **g)
                .count();
            correct as f64 / labels_gt.len().max(1) as f64
        };

        // ── Anti-lie: accounting identity cross-check at binary level too ───────
        debug_assert_eq!(
            automl_plan.selected.len()
                + automl_plan.signals_rejected_correlation
                + automl_plan.signals_rejected_no_gain,
            automl_plan.signals_evaluated,
            "pdc2025: HDIT accounting identity violated",
        );

        // ── HDIT AutoML plan metadata (per-log): log and persist ──────────────────
        info!(
            "  [{}] HDIT plan: fusion={:?} selected={:?} tiers={:?} evaluated={} rejected_corr={} rejected_gain={} timing={}μs | anchor_vs_gt={:.2}% plan_vs_gt={:.2}% oracle={}@{:.2}% gap={:+.2}%",
            stem,
            automl_plan.fusion,
            automl_plan.selected,
            automl_plan.tiers.iter().map(|(n, t)| format!("{}:{}", n, t.label())).collect::<Vec<_>>(),
            automl_plan.signals_evaluated,
            automl_plan.signals_rejected_correlation,
            automl_plan.signals_rejected_no_gain,
            automl_plan.total_timing_us,
            anchor_vs_gt * 100.0,
            plan_vs_gt * 100.0,
            oracle.0,
            oracle.1 * 100.0,
            (plan_vs_gt - oracle.1) * 100.0,
        );

        // Persist AutomlPlan + anti-lie metrics to JSON for longitudinal analysis
        let plan_json = serde_json::json!({
            "log": stem,
            "log_idx": log_idx,
            "n_traces": n_t,
            "fusion": format!("{:?}", automl_plan.fusion),
            "selected": automl_plan.selected,
            "tiers": automl_plan.tiers.iter()
                .map(|(n, t)| serde_json::json!({"signal": n, "tier": t.label()}))
                .collect::<Vec<_>>(),
            "plan_accuracy_vs_anchor": automl_plan.plan_accuracy,
            "plan_accuracy_vs_gt": plan_vs_gt,
            "anchor_vs_gt": anchor_vs_gt,
            "oracle_signal": oracle.0,
            "oracle_vs_gt": oracle.1,
            "oracle_gap": plan_vs_gt - oracle.1,
            "per_signal_gt_accuracy": per_signal_gt_acc.iter()
                .map(|(n, a)| serde_json::json!({"signal": n, "acc_vs_gt": a}))
                .collect::<Vec<_>>(),
            "total_timing_us": automl_plan.total_timing_us,
            "signals_evaluated": automl_plan.signals_evaluated,
            "signals_rejected_correlation": automl_plan.signals_rejected_correlation,
            "signals_rejected_no_gain": automl_plan.signals_rejected_no_gain,
            "accounting_balanced": automl_plan.selected.len()
                + automl_plan.signals_rejected_correlation
                + automl_plan.signals_rejected_no_gain
                == automl_plan.signals_evaluated,
        });
        // Anti-lie: persistence failures MUST surface, not be swallowed.
        // A silent write failure = a silent lie about what was produced.
        let plan_dir = output_dir.join("automl_plans");
        std::fs::create_dir_all(&plan_dir)
            .unwrap_or_else(|e| panic!("Failed to create plan dir {:?}: {}", plan_dir, e));
        let plan_json_str = serde_json::to_string_pretty(&plan_json)
            .expect("plan_json serialization cannot fail — all fields are pre-validated");
        std::fs::write(plan_dir.join(format!("{}.json", stem)), plan_json_str)
            .unwrap_or_else(|e| panic!("Failed to write plan for {}: {}", stem, e));

        // ── Comprehensive signal fusion (all strategies pooled) ─────────────────────
        let n_traces = log.traces.len();

        // Collect ALL bool signals into one pool
        let all_bool_signals: Vec<Vec<bool>> = vec![
            cls_f.clone(),
            cls_g.clone(),
            cls_h.clone(),
            cls_combo.clone(),
            cls_vote500.clone(),
            cls_s_ensemble.clone(),
            cls_e.clone(),
        ];

        // Continuous score signals (higher = more positive)
        let score_neg_edit: Vec<f64> = edit_dist_to_score(&edit_dists_global);
        let score_f_bool: Vec<f64> = bool_to_score(&cls_f);
        let score_e_bool: Vec<f64> = bool_to_score(&cls_e);
        let score_signals: Vec<Vec<f64>> = vec![score_neg_edit, score_f_bool, score_e_bool];

        // Anchor proxy: trace predicted positive by at least 4 of 7 bool signals
        let anchor_proxy: Vec<bool> = (0..n_traces)
            .map(|i| all_bool_signals.iter().filter(|s| s[i]).count() >= 4)
            .collect();

        // Build combined signal pool (score + bool) for rank-fusion methods
        let combined_signals: Vec<Vec<f64>> = {
            let mut s = score_signals.clone();
            s.extend(all_bool_signals.iter().map(|p| bool_to_score(p)));
            s
        };
        let combined_hib: Vec<bool> = vec![true; combined_signals.len()];

        // 1. Borda count fusion
        let cls_borda = borda_count(&combined_signals, &combined_hib, 500);

        // 2. Reciprocal rank fusion
        let cls_rrf = reciprocal_rank_fusion(&combined_signals, &combined_hib, 500);

        // 3. Weighted vote (anchor = majority of signals)
        let cls_weighted = auto_weighted_vote(&all_bool_signals, &anchor_proxy, 500);

        // 4. Precision-weighted vote
        let cls_prec_weighted = precision_weighted_vote(&all_bool_signals, &anchor_proxy, 500);

        // 5. Stacked meta-learner
        let cls_stacked = stack_ensemble(&all_bool_signals, &anchor_proxy, 500);

        // 6. Full combinatorial (bool + continuous)
        let cls_full_combo =
            full_combinatorial(&all_bool_signals, &score_signals, &anchor_proxy, 500);

        // 7. Best bool+score pair
        let cls_best_pair =
            best_bool_score_pair(&all_bool_signals, &score_signals, &anchor_proxy, 500);

        // Suppress unused import warnings for stack_logistic / stack_tree
        // (stack_ensemble calls them internally; direct calls kept for completeness)
        let _ = stack_logistic as fn(&[Vec<bool>], &[bool], usize) -> Vec<bool>;
        let _ = stack_tree as fn(&[Vec<bool>], &[bool], usize) -> Vec<bool>;

        // ── Ensemble combinations ─────────────────────────────────────────────

        // Majority vote of F+G+H (2-of-3 wins)
        let cls_fgh: Vec<bool> = (0..log.traces.len())
            .map(|i| {
                [cls_f[i], cls_g[i], cls_h[i]]
                    .iter()
                    .filter(|&&b| b)
                    .count()
                    >= 2
            })
            .collect();

        // F OR G (union — high recall)
        let cls_fg_or: Vec<bool> = cls_f
            .iter()
            .zip(cls_g.iter())
            .map(|(&a, &b)| a || b)
            .collect();

        // F AND G (intersection — high precision)
        let cls_fg_and: Vec<bool> = cls_f
            .iter()
            .zip(cls_g.iter())
            .map(|(&a, &b)| a && b)
            .collect();

        // A OR any-non-GT (GT always wins, shows GT dominates)
        let cls_a_or_fgh: Vec<bool> = (0..log.traces.len())
            .map(|i| cls_a[i] || cls_fgh[i])
            .collect();

        // ── Score all ────────────────────────────────────────────────────────
        let n = log.traces.len();
        for i in 0..n {
            let gt_lbl = labels_gt.get(i).copied().unwrap_or(false);
            acc.a += (cls_a[i] == gt_lbl) as usize;
            acc.b += (cls_b[i] == gt_lbl) as usize;
            acc.c += (cls_c[i] == gt_lbl) as usize;
            acc.d += (cls_d[i] == gt_lbl) as usize;
            acc.e += (cls_e[i] == gt_lbl) as usize;
            acc.f += (cls_f[i] == gt_lbl) as usize;
            acc.g += (cls_g[i] == gt_lbl) as usize;
            acc.h += (cls_h[i] == gt_lbl) as usize;
            acc.hdc += (cls_hdc[i] == gt_lbl) as usize;
            acc.automl += (cls_automl[i] == gt_lbl) as usize;
            acc.rl_automl += (cls_rl_automl[i] == gt_lbl) as usize;
            acc.combinator += (cls_combinator[i] == gt_lbl) as usize;
            acc.sup_trained += (cls_sup_trained[i] == gt_lbl) as usize;
            acc.automl_hyper += (cls_automl_hyper[i] == gt_lbl) as usize;
            acc.fgh += (cls_fgh[i] == gt_lbl) as usize;
            acc.fg_or += (cls_fg_or[i] == gt_lbl) as usize;
            acc.fg_and += (cls_fg_and[i] == gt_lbl) as usize;
            acc.a_or_fgh += (cls_a_or_fgh[i] == gt_lbl) as usize;
            acc.combo += (cls_combo[i] == gt_lbl) as usize;
            acc.vote500 += (cls_vote500[i] == gt_lbl) as usize;
            acc.s_ensemble += (cls_s_ensemble[i] == gt_lbl) as usize;
            acc.s_knn += (cls_s_knn[i] == gt_lbl) as usize;
            acc.s_tree += (cls_s_tree[i] == gt_lbl) as usize;
            acc.s_nb += (cls_s_nb[i] == gt_lbl) as usize;
            acc.borda += (cls_borda[i] == gt_lbl) as usize;
            acc.rrf += (cls_rrf[i] == gt_lbl) as usize;
            acc.weighted += (cls_weighted[i] == gt_lbl) as usize;
            acc.prec_weighted += (cls_prec_weighted[i] == gt_lbl) as usize;
            acc.stacked += (cls_stacked[i] == gt_lbl) as usize;
            acc.full_combo += (cls_full_combo[i] == gt_lbl) as usize;
            acc.best_pair += (cls_best_pair[i] == gt_lbl) as usize;
            acc.total += 1;
        }

        // ── Per-log best-strategy output (anchor_proxy as quality proxy) ──────
        let strategy_pool: &[(&str, &Vec<bool>)] = &[
            ("f", &cls_f),
            ("g", &cls_g),
            ("h", &cls_h),
            ("hdc", &cls_hdc),
            ("combinator", &cls_combinator),
            ("automl", &cls_automl),
            ("sup_trained", &cls_sup_trained),
            ("borda", &cls_borda),
            ("rrf", &cls_rrf),
            ("weighted", &cls_weighted),
            ("prec_weighted", &cls_prec_weighted),
            ("stacked", &cls_stacked),
            ("full_combo", &cls_full_combo),
            ("best_pair", &cls_best_pair),
            ("combo", &cls_combo),
            ("vote500", &cls_vote500),
            ("e", &cls_e),
            ("s_ensemble", &cls_s_ensemble),
        ];
        let (best_name, best_preds, best_sc) = strategy_pool
            .iter()
            .map(|(name, preds)| (*name, *preds, score(preds, &anchor_proxy, 500)))
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(("automl", &cls_automl, 0.0));
        acc.best_per_log += best_preds
            .iter()
            .zip(labels_gt.iter())
            .filter(|(&p, &g)| p == g)
            .count();
        info!(
            "  [{}] best={} anchor_score={:.3}",
            stem, best_name, best_sc
        );
        let _ = write_classified_log(&log, best_preds, &output_dir.join(format!("{}.xes", stem)));
    }

    let t = acc.total as f64;
    info!(
        "\n=== PDC 2025 — All strategies attached ({} traces across 96 logs) ===",
        acc.total
    );
    info!("");
    info!("── GT strategies (require answer key) ──────────────────────────────");
    info!(
        "  A  Vec<bool> positional:          {:.2}%  ~1 ns/trace",
        acc.a as f64 / t * 100.0
    );
    info!(
        "  B  u64 bitmask:                   {:.2}%  ~1 ns/trace",
        acc.b as f64 / t * 100.0
    );
    info!(
        "  C  FxHashMap by trace name:       {:.2}%  ~5 ns/trace",
        acc.c as f64 / t * 100.0
    );
    info!(
        "  D  FNV activity-seq hash:         {:.2}%  ~5 ns/trace  (336 ambiguous seqs)",
        acc.d as f64 / t * 100.0
    );
    info!("");
    info!("── Conformance strategies (no GT) ──────────────────────────────────");
    info!(
        "  F  classify_exact (in_lang+fill): {:.2}%",
        acc.f as f64 / t * 100.0
    );
    info!(
        "  G  fitness top-500:               {:.2}%",
        acc.g as f64 / t * 100.0
    );
    info!(
        "  H  in_language + fitness fill:    {:.2}%",
        acc.h as f64 / t * 100.0
    );
    info!("── Strategy HDC: Hyperdimensional trace classification ─────────────────");
    info!(
        "  HDC hypervector prototype:        {:.2}%",
        acc.hdc as f64 / t * 100.0
    );
    info!("── Strategy T: Training-data supervised transfer ───────────────────────────");
    info!(
        "  T.sup_trained vote:              {:.2}%",
        acc.sup_trained as f64 / t * 100.0
    );
    info!("── Strategy AutoML: RL hyperparameter search + HDIT signal selection ──────");
    info!(
        "  RL AutoML (best net):             {:.2}%",
        acc.rl_automl as f64 / t * 100.0
    );
    info!(
        "  AutoML orthogonal select:         {:.2}%",
        acc.automl as f64 / t * 100.0
    );
    info!("");
    info!("── Ensembles ────────────────────────────────────────────────────────");
    info!(
        "  F∨G∨H majority vote (2/3):        {:.2}%",
        acc.fgh as f64 / t * 100.0
    );
    info!(
        "  F ∪ G  (union):                   {:.2}%",
        acc.fg_or as f64 / t * 100.0
    );
    info!(
        "  F ∩ G  (intersection):             {:.2}%",
        acc.fg_and as f64 / t * 100.0
    );
    info!(
        "  A ∪ F∨G∨H (GT dominates):         {:.2}%",
        acc.a_or_fgh as f64 / t * 100.0
    );
    info!("");
    info!("── ML Ensemble strategies ──────────────────────────────────────────────");
    info!(
        "  Combo (combinatorial search):    {:.2}%",
        acc.combo as f64 / t * 100.0
    );
    info!(
        "  Vote500 (all classifiers top500): {:.2}%",
        acc.vote500 as f64 / t * 100.0
    );
    info!("── Strategy S: Synthetic ML (trained on net-generated data) ─────────────");
    info!(
        "  S.knn      k-NN on synthetic:         {:.2}%",
        acc.s_knn as f64 / t * 100.0
    );
    info!(
        "  S.nb       Naive Bayes on synthetic:   {:.2}%",
        acc.s_nb as f64 / t * 100.0
    );
    info!(
        "  S.tree     Decision Tree on synthetic: {:.2}%",
        acc.s_tree as f64 / t * 100.0
    );
    info!(
        "  S.ensemble Majority vote ensemble:     {:.2}%",
        acc.s_ensemble as f64 / t * 100.0
    );
    info!("── Strategy E: Edit-distance k-NN on enumerated language ───────────────");
    info!(
        "  E  edit-dist top-500:              {:.2}%",
        acc.e as f64 / t * 100.0
    );
    info!("── Comprehensive Signal Fusion (all signals pooled) ─────────────────────");
    info!(
        "  Borda count fusion:              {:.2}%",
        acc.borda as f64 / t * 100.0
    );
    info!(
        "  Reciprocal rank fusion:          {:.2}%",
        acc.rrf as f64 / t * 100.0
    );
    info!(
        "  Weighted vote:                   {:.2}%",
        acc.weighted as f64 / t * 100.0
    );
    info!(
        "  Precision-weighted vote:         {:.2}%",
        acc.prec_weighted as f64 / t * 100.0
    );
    info!(
        "  Stacked meta-learner:            {:.2}%",
        acc.stacked as f64 / t * 100.0
    );
    info!(
        "  Full combinatorial (bool+score): {:.2}%",
        acc.full_combo as f64 / t * 100.0
    );
    info!(
        "  Best bool+score pair:            {:.2}%",
        acc.best_pair as f64 / t * 100.0
    );
    info!(
        "  run_combinator (20-clf pool):    {:.2}%",
        acc.combinator as f64 / t * 100.0
    );
    info!(
        "  AutoML hyper-sweep:              {:.2}%",
        acc.automl_hyper as f64 / t * 100.0
    );
    info!("── Best-per-log (anchor proxy) ─────────────────────────────────────────");
    info!(
        "  Best strategy per log:           {:.2}%",
        acc.best_per_log as f64 / t * 100.0
    );

    // ── Aggregate AutoML plan summary (re-reads JSON files — CANNOT be fabricated) ──
    // Anti-lie: this summary is derived ONLY from the on-disk plan artifacts.
    // In-memory state cannot inject false numbers here.
    let plan_dir = output_dir.join("automl_plans");
    if let Ok(entries) = std::fs::read_dir(&plan_dir) {
        let mut plan_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
            .collect();
        plan_files.sort();

        let mut signal_selection_count: FxHashMap<String, usize> = FxHashMap::default();
        let mut fusion_op_count: FxHashMap<String, usize> = FxHashMap::default();
        let mut tier_count: FxHashMap<String, usize> = FxHashMap::default();
        let mut total_evaluated = 0usize;
        let mut total_rejected_corr = 0usize;
        let mut total_rejected_gain = 0usize;
        let mut total_selected = 0usize;
        let mut sum_plan_vs_gt = 0.0f64;
        let mut sum_anchor_vs_gt = 0.0f64;
        let mut sum_oracle_vs_gt = 0.0f64;
        let mut sum_oracle_gap = 0.0f64;
        let mut n_plans_counted = 0usize;
        let mut n_accounting_violations = 0usize;
        #[allow(unused_assignments)]
        for path in &plan_files {
            // Anti-lie: read errors and parse errors are themselves lies about
            // what was produced. Panic instead of continuing past them.
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read plan {:?}: {}", path, e));
            let plan: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Corrupted plan JSON {:?}: {}", path, e));

            // Anti-lie: accounting_balanced==false is an invariant violation produced by
            // THIS binary. We wrote it; if it's false, the pipeline is broken.
            let balanced = plan.get("accounting_balanced").and_then(|v| v.as_bool());
            match balanced {
                Some(true) => {}
                Some(false) => {
                    n_accounting_violations += 1;
                    panic!(
                        "Invariant violation: plan {:?} has accounting_balanced=false — HDIT bookkeeping is broken",
                        path,
                    );
                }
                None => panic!(
                    "Plan {:?} missing accounting_balanced field — old/corrupted format",
                    path,
                ),
            }

            if let Some(selected) = plan.get("selected").and_then(|v| v.as_array()) {
                for sig in selected {
                    if let Some(name) = sig.as_str() {
                        *signal_selection_count.entry(name.to_string()).or_insert(0) += 1;
                    }
                }
                total_selected += selected.len();
            }
            if let Some(fusion) = plan.get("fusion").and_then(|v| v.as_str()) {
                *fusion_op_count.entry(fusion.to_string()).or_insert(0) += 1;
            }
            if let Some(tiers) = plan.get("tiers").and_then(|v| v.as_array()) {
                for t in tiers {
                    if let Some(tier) = t.get("tier").and_then(|v| v.as_str()) {
                        *tier_count.entry(tier.to_string()).or_insert(0) += 1;
                    }
                }
            }
            total_evaluated += plan
                .get("signals_evaluated")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            total_rejected_corr += plan
                .get("signals_rejected_correlation")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            total_rejected_gain += plan
                .get("signals_rejected_no_gain")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            sum_plan_vs_gt += plan
                .get("plan_accuracy_vs_gt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            sum_anchor_vs_gt += plan
                .get("anchor_vs_gt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            sum_oracle_vs_gt += plan
                .get("oracle_vs_gt")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            sum_oracle_gap += plan
                .get("oracle_gap")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            n_plans_counted += 1;
        }

        // Anti-lie aggregate invariant: accounting must balance globally too
        assert_eq!(
            total_selected + total_rejected_corr + total_rejected_gain,
            total_evaluated,
            "Aggregate accounting lie: selected({}) + rej_corr({}) + rej_gain({}) != evaluated({})",
            total_selected,
            total_rejected_corr,
            total_rejected_gain,
            total_evaluated,
        );

        let n_f = n_plans_counted.max(1) as f64;
        let mut sig_freq: Vec<(String, usize)> = signal_selection_count.into_iter().collect();
        sig_freq.sort_by_key(|b| std::cmp::Reverse(b.1));
        let mut fusion_freq: Vec<(String, usize)> = fusion_op_count.into_iter().collect();
        fusion_freq.sort_by_key(|b| std::cmp::Reverse(b.1));
        let mut tier_freq: Vec<(String, usize)> = tier_count.into_iter().collect();
        tier_freq.sort_by_key(|b| std::cmp::Reverse(b.1));

        info!("");
        info!(
            "── AutoML Aggregate Summary (from {} JSON plans) ────────────────────",
            n_plans_counted
        );
        info!(
            "  avg plan accuracy vs GT:         {:.2}%",
            sum_plan_vs_gt / n_f * 100.0
        );
        info!(
            "  avg anchor accuracy vs GT:       {:.2}%",
            sum_anchor_vs_gt / n_f * 100.0
        );
        info!(
            "  avg oracle (best-signal) vs GT:  {:.2}%",
            sum_oracle_vs_gt / n_f * 100.0
        );
        info!(
            "  avg HDIT gap vs oracle:          {:+.2}%  (negative = HDIT underperformed oracle)",
            sum_oracle_gap / n_f * 100.0
        );
        info!("  signals evaluated (total):       {}", total_evaluated);
        info!(
            "  selected / rejected_corr / rejected_gain: {} / {} / {}",
            total_selected, total_rejected_corr, total_rejected_gain
        );
        info!(
            "  accounting violations (dropped): {}",
            n_accounting_violations
        );
        info!("  signal selection frequency:");
        for (name, count) in sig_freq.iter().take(12) {
            info!(
                "    {:<24} {} ({:.1}%)",
                name,
                count,
                *count as f64 / n_f * 100.0
            );
        }
        info!("  fusion op frequency:");
        for (name, count) in fusion_freq.iter() {
            info!(
                "    {:<24} {} ({:.1}%)",
                name,
                count,
                *count as f64 / n_f * 100.0
            );
        }
        info!("  tier frequency (across selected signals):");
        for (name, count) in tier_freq.iter() {
            info!("    {:<24} {}", name, count);
        }

        // Persist aggregate summary to JSON
        let summary_json = serde_json::json!({
            "n_plans_counted": n_plans_counted,
            "n_accounting_violations": n_accounting_violations,
            "avg_plan_vs_gt": sum_plan_vs_gt / n_f,
            "avg_anchor_vs_gt": sum_anchor_vs_gt / n_f,
            "avg_oracle_vs_gt": sum_oracle_vs_gt / n_f,
            "avg_oracle_gap": sum_oracle_gap / n_f,
            "total_signals_evaluated": total_evaluated,
            "total_selected": total_selected,
            "total_rejected_correlation": total_rejected_corr,
            "total_rejected_no_gain": total_rejected_gain,
            "signal_selection_frequency": sig_freq,
            "fusion_op_frequency": fusion_freq,
            "tier_frequency": tier_freq,
            "accounting_balanced_global":
                total_selected + total_rejected_corr + total_rejected_gain == total_evaluated,
        });
        // Anti-lie: summary serialization MUST succeed — all fields are plain f64/usize.
        let summary_str = serde_json::to_string_pretty(&summary_json)
            .expect("summary_json serialization cannot fail — all fields are primitives");
        let summary_path = output_dir.join("automl_summary.json");
        std::fs::write(&summary_path, summary_str)
            .unwrap_or_else(|e| panic!("Failed to write summary {:?}: {}", summary_path, e));

        // Anti-lie: if we had ANY accounting violations, refuse to return success.
        // The panic above would have fired first, but belt-and-suspenders.
        assert_eq!(
            n_accounting_violations, 0,
            "AutoML summary completed with {} accounting violations — aggregate is a lie",
            n_accounting_violations,
        );
    }
}

#[derive(Default)]
struct Acc {
    total: usize,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    g: usize,
    h: usize,
    hdc: usize,
    automl: usize,
    rl_automl: usize,
    fgh: usize,
    fg_or: usize,
    fg_and: usize,
    a_or_fgh: usize,
    combo: usize,
    vote500: usize,
    s_ensemble: usize,
    s_knn: usize,
    s_tree: usize,
    s_nb: usize,
    borda: usize,
    rrf: usize,
    weighted: usize,
    prec_weighted: usize,
    stacked: usize,
    full_combo: usize,
    best_pair: usize,
    combinator: usize,
    sup_trained: usize,
    automl_hyper: usize,
    best_per_log: usize,
}

fn seq_hash(trace: &dteam::models::Trace) -> u64 {
    let mut buf: Vec<u8> = Vec::with_capacity(trace.events.len() * 8);
    for event in &trace.events {
        if let Some(name) = event
            .attributes
            .iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| {
                if let AttributeValue::String(s) = &a.value {
                    Some(s.as_bytes())
                } else {
                    None
                }
            })
        {
            buf.extend_from_slice(name);
            buf.push(0);
        }
    }
    fnv1a_64(&buf)
}

fn rank_top(scores: &[f64], n: usize) -> Vec<bool> {
    let mut idx: Vec<(usize, f64)> = scores.iter().enumerate().map(|(i, &s)| (i, s)).collect();
    idx.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    let mut out = vec![false; scores.len()];
    for &(i, _) in idx.iter().take(n) {
        out[i] = true;
    }
    out
}
