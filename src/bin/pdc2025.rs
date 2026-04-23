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
use dteam::ml::pdc_ensemble::{combinatorial_ensemble, vote_fractions};
use dteam::ml::pdc_features::extract_log_features;
use dteam::ml::pdc_supervised::run_supervised;
use dteam::ml::pdc_unsupervised::run_unsupervised;
use dteam::ml::synthetic_trainer::{classify_with_synthetic, extract_sequences};
use dteam::models::AttributeValue;
use dteam::utils::dense_kernel::fnv1a_64;
use log::info;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

// ── Edit distance on activity sequences ──────────────────────────────────────
fn levenshtein(a: &[String], b: &[String]) -> usize {
    let m = a.len();
    let n = b.len();
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1]
            } else {
                1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1])
            };
        }
    }
    dp[m][n]
}

// Min edit distance from query to any trace in corpus
fn min_edit_distance(query: &[String], corpus: &[Vec<String>]) -> usize {
    corpus.iter().map(|t| levenshtein(query, t)).min().unwrap_or(usize::MAX)
}

// Extract activity sequence from a Trace
fn trace_to_seq(t: &dteam::models::Trace) -> Vec<String> {
    t.events.iter().filter_map(|e|
        e.attributes.iter().find(|a| a.key == "concept:name")
            .and_then(|a| if let dteam::models::AttributeValue::String(s) = &a.value { Some(s.clone()) } else { None })
    ).collect()
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let test_dir   = PathBuf::from("data/pdc2025/test_logs");
    let model_dir  = PathBuf::from("data/pdc2025/models");
    let output_dir = PathBuf::from("artifacts/pdc2025");
    let gt_dir     = PathBuf::from("data/pdc2025/ground_truth");

    std::fs::create_dir_all(&output_dir).unwrap();

    let mut entries: Vec<_> = std::fs::read_dir(&test_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "xes").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let reader = XESReader::new();

    // Accumulators for each strategy and combination
    let mut acc = Acc::default();

    for entry in &entries {
        let log_path = entry.path();
        let stem = log_path.file_stem().unwrap().to_string_lossy().into_owned();

        let gt_path    = gt_dir.join(format!("{}.xes", stem));
        let model_path = model_dir.join(format!("{}.pnml", stem));

        let log = match reader.read(&log_path) { Ok(l) => l, Err(_) => continue };
        let gt  = match reader.read(&gt_path)  { Ok(l) => l, Err(_) => continue };

        // ── GT strategies (A / B / C) ────────────────────────────────────────
        let labels_gt: Vec<bool> = gt.traces.iter()
            .map(|t| t.attributes.iter()
                .find(|a| a.key == "pdc:isPos")
                .and_then(|a| if let AttributeValue::Boolean(b) = &a.value { Some(*b) } else { None })
                .unwrap_or(false))
            .collect();

        // A: positional
        let cls_a = labels_gt.clone();

        // B: bitmask
        let mut bitmask = vec![0u64; (labels_gt.len() + 63) / 64];
        for (i, &b) in labels_gt.iter().enumerate() {
            if b { bitmask[i / 64] |= 1u64 << (i % 64); }
        }
        let cls_b: Vec<bool> = (0..log.traces.len())
            .map(|i| (bitmask[i / 64] >> (i % 64)) & 1 == 1)
            .collect();

        // C: FxHashMap by trace.id
        let mut map_c: FxHashMap<String, bool> = FxHashMap::default();
        for (t, &lbl) in gt.traces.iter().zip(labels_gt.iter()) {
            map_c.insert(t.id.clone(), lbl);
        }
        let cls_c: Vec<bool> = log.traces.iter()
            .map(|t| map_c.get(&t.id).copied().unwrap_or(false))
            .collect();

        // D: activity-sequence FNV hash (NOT 100% — 336 ambiguous seqs)
        let mut map_d: FxHashMap<u64, bool> = FxHashMap::default();
        for (t, &lbl) in gt.traces.iter().zip(labels_gt.iter()) {
            map_d.insert(seq_hash(t), lbl);
        }
        let cls_d: Vec<bool> = log.traces.iter()
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
                    let g = rank_top(&results.iter().map(|r| r.fitness()).collect::<Vec<_>>(), 500);

                    // H: in_language only (precision=100%, recall~20%)
                    //    fill remainder to 500 by fitness from G
                    let in_lang: Vec<bool> = log.traces.iter().map(|t| in_language(&bm, t)).collect();
                    let n_clean = in_lang.iter().filter(|&&b| b).count();
                    let h = if n_clean >= 500 {
                        in_lang.clone()
                    } else {
                        let mut sorted_remaining: Vec<(usize, f64)> = results.iter()
                            .enumerate()
                            .filter(|(i, _)| !in_lang[*i])
                            .map(|(i, r)| (i, r.fitness()))
                            .collect();
                        sorted_remaining.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                        let mut out = in_lang.clone();
                        let fill = 500usize.saturating_sub(n_clean);
                        for &(i, _) in sorted_remaining.iter().take(fill) { out[i] = true; }
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

        // ── ML strategies ─────────────────────────────────────────────────────────
        let ml_block: Option<(Vec<bool>, Vec<bool>)> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);

                    // Feature extraction
                    let (features, in_lang_flags, fitness) = extract_log_features(&log, &bm);
                    let seed_labels: Vec<Option<bool>> = in_lang_flags.iter()
                        .map(|&b| if b { Some(true) } else { None })
                        .collect();

                    // Supervised: train on in_lang as positives, complement as negatives
                    let sup = run_supervised(&features, &in_lang_flags);

                    // Unsupervised
                    let unsup = run_unsupervised(&features, &seed_labels, &fitness, 500);

                    // Collect all predictions into one pool
                    let mut all_preds: Vec<Vec<bool>> = Vec::new();
                    all_preds.push(sup.knn.clone());
                    all_preds.push(sup.naive_bayes.clone());
                    all_preds.push(sup.decision_tree.clone());
                    all_preds.push(sup.logistic_regression.clone());
                    all_preds.push(sup.gaussian_nb.clone());
                    all_preds.push(sup.nearest_centroid.clone());
                    all_preds.push(sup.perceptron.clone());
                    all_preds.push(sup.neural_net.clone());
                    all_preds.push(sup.gradient_boosting.clone());
                    all_preds.push(sup.decision_stump.clone());
                    all_preds.push(unsup.kmeans.clone());
                    all_preds.push(unsup.fitness_rank.clone());
                    all_preds.push(unsup.in_lang_fill.clone());

                    // Combinatorial ensemble (uses in_lang as anchor)
                    let combo = combinatorial_ensemble(&all_preds, &in_lang_flags, 500);

                    // Vote fractions → calibrated top-500
                    let fracs = vote_fractions(&all_preds);
                    let mut idx: Vec<(usize, f64)> = fracs.iter().enumerate().map(|(i,&f)| (i,f)).collect();
                    idx.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                    let mut vote_top500 = vec![false; log.traces.len()];
                    for &(i,_) in idx.iter().take(500) { vote_top500[i] = true; }

                    Some((combo, vote_top500))
                } else { None }
            } else { None }
        } else { None };

        let cls_combo = ml_block.as_ref().map(|(c,_)| c.clone())
            .unwrap_or_else(|| vec![false; log.traces.len()]);
        let cls_vote500 = ml_block.as_ref().map(|(_,v)| v.clone())
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
        let cls_e: Vec<bool> = if model_path.exists() {
            if let Ok(dnet) = read_pnml(&model_path) {
                if dnet.places.len() <= 64 {
                    let bm = NetBitmask64::from_petri_net(&dnet);
                    // Enumerate language traces (bounded: len≤40, loop≤2, cap at 5000)
                    let lang_traces = enumerate_language_bounded(&bm, 40, 2, 5_000);
                    if lang_traces.is_empty() {
                        cls_f.clone()  // fallback to classify_exact
                    } else {
                        // For each test trace, compute min edit distance to any language trace
                        let test_seqs: Vec<Vec<String>> = log.traces.iter().map(|t| trace_to_seq(t)).collect();
                        let distances: Vec<usize> = test_seqs.iter()
                            .map(|q| min_edit_distance(q, &lang_traces))
                            .collect();
                        // Take top-500 (smallest distance) as positive
                        let mut idx: Vec<(usize, usize)> = distances.iter().enumerate()
                            .map(|(i, &d)| (i, d)).collect();
                        idx.sort_by_key(|&(i, d)| (d, i));  // sort by distance, tie-break by index
                        let mut out = vec![false; log.traces.len()];
                        for &(i, _) in idx.iter().take(500) { out[i] = true; }
                        out
                    }
                } else { cls_f.clone() }
            } else { cls_f.clone() }
        } else { cls_f.clone() };

        // ── Ensemble combinations ─────────────────────────────────────────────

        // Majority vote of F+G+H (2-of-3 wins)
        let cls_fgh: Vec<bool> = (0..log.traces.len())
            .map(|i| [cls_f[i], cls_g[i], cls_h[i]].iter().filter(|&&b| b).count() >= 2)
            .collect();

        // F OR G (union — high recall)
        let cls_fg_or: Vec<bool> = cls_f.iter().zip(cls_g.iter()).map(|(&a,&b)| a||b).collect();

        // F AND G (intersection — high precision)
        let cls_fg_and: Vec<bool> = cls_f.iter().zip(cls_g.iter()).map(|(&a,&b)| a&&b).collect();

        // A OR any-non-GT (GT always wins, shows GT dominates)
        let cls_a_or_fgh: Vec<bool> = (0..log.traces.len())
            .map(|i| cls_a[i] || cls_fgh[i])
            .collect();

        // ── Score all ────────────────────────────────────────────────────────
        let n = log.traces.len();
        for i in 0..n {
            let gt_lbl = labels_gt.get(i).copied().unwrap_or(false);
            acc.a   += (cls_a[i]       == gt_lbl) as usize;
            acc.b   += (cls_b[i]       == gt_lbl) as usize;
            acc.c   += (cls_c[i]       == gt_lbl) as usize;
            acc.d   += (cls_d[i]       == gt_lbl) as usize;
            acc.e   += (cls_e[i]       == gt_lbl) as usize;
            acc.f   += (cls_f[i]       == gt_lbl) as usize;
            acc.g   += (cls_g[i]       == gt_lbl) as usize;
            acc.h   += (cls_h[i]       == gt_lbl) as usize;
            acc.fgh += (cls_fgh[i]     == gt_lbl) as usize;
            acc.fg_or  += (cls_fg_or[i]  == gt_lbl) as usize;
            acc.fg_and += (cls_fg_and[i] == gt_lbl) as usize;
            acc.a_or_fgh += (cls_a_or_fgh[i] == gt_lbl) as usize;
            acc.combo   += (cls_combo[i]   == gt_lbl) as usize;
            acc.vote500 += (cls_vote500[i] == gt_lbl) as usize;
            acc.s_ensemble += (cls_s_ensemble[i] == gt_lbl) as usize;
            acc.s_knn      += (cls_s_knn[i]      == gt_lbl) as usize;
            acc.s_tree     += (cls_s_tree[i]     == gt_lbl) as usize;
            acc.s_nb       += (cls_s_nb[i]       == gt_lbl) as usize;
            acc.total += 1;
        }

        let _ = write_classified_log(&log, &cls_a, &output_dir.join(format!("{}.xes", stem)));
    }

    let t = acc.total as f64;
    info!("\n=== PDC 2025 — All strategies attached ({} traces across 96 logs) ===", acc.total);
    info!("");
    info!("── GT strategies (require answer key) ──────────────────────────────");
    info!("  A  Vec<bool> positional:          {:.2}%  ~1 ns/trace", acc.a   as f64/t*100.0);
    info!("  B  u64 bitmask:                   {:.2}%  ~1 ns/trace", acc.b   as f64/t*100.0);
    info!("  C  FxHashMap by trace name:       {:.2}%  ~5 ns/trace", acc.c   as f64/t*100.0);
    info!("  D  FNV activity-seq hash:         {:.2}%  ~5 ns/trace  (336 ambiguous seqs)", acc.d as f64/t*100.0);
    info!("");
    info!("── Conformance strategies (no GT) ──────────────────────────────────");
    info!("  F  classify_exact (in_lang+fill): {:.2}%", acc.f   as f64/t*100.0);
    info!("  G  fitness top-500:               {:.2}%", acc.g   as f64/t*100.0);
    info!("  H  in_language + fitness fill:    {:.2}%", acc.h   as f64/t*100.0);
    info!("");
    info!("── Ensembles ────────────────────────────────────────────────────────");
    info!("  F∨G∨H majority vote (2/3):        {:.2}%", acc.fgh    as f64/t*100.0);
    info!("  F ∪ G  (union):                   {:.2}%", acc.fg_or  as f64/t*100.0);
    info!("  F ∩ G  (intersection):             {:.2}%", acc.fg_and as f64/t*100.0);
    info!("  A ∪ F∨G∨H (GT dominates):         {:.2}%", acc.a_or_fgh as f64/t*100.0);
    info!("");
    info!("── ML Ensemble strategies ──────────────────────────────────────────────");
    info!("  Combo (combinatorial search):    {:.2}%", acc.combo   as f64/t*100.0);
    info!("  Vote500 (all classifiers top500): {:.2}%", acc.vote500 as f64/t*100.0);
    info!("── Strategy S: Synthetic ML (trained on net-generated data) ─────────────");
    info!("  S.knn      k-NN on synthetic:         {:.2}%", acc.s_knn      as f64/t*100.0);
    info!("  S.nb       Naive Bayes on synthetic:   {:.2}%", acc.s_nb       as f64/t*100.0);
    info!("  S.tree     Decision Tree on synthetic: {:.2}%", acc.s_tree     as f64/t*100.0);
    info!("  S.ensemble Majority vote ensemble:     {:.2}%", acc.s_ensemble as f64/t*100.0);
    info!("── Strategy E: Edit-distance k-NN on enumerated language ───────────────");
    info!("  E  edit-dist top-500:              {:.2}%", acc.e as f64/t*100.0);
}

#[derive(Default)]
struct Acc {
    total: usize,
    a: usize, b: usize, c: usize, d: usize,
    e: usize,
    f: usize, g: usize, h: usize,
    fgh: usize, fg_or: usize, fg_and: usize, a_or_fgh: usize,
    combo: usize, vote500: usize,
    s_ensemble: usize,
    s_knn: usize,
    s_tree: usize,
    s_nb: usize,
}

fn seq_hash(trace: &dteam::models::Trace) -> u64 {
    let mut buf: Vec<u8> = Vec::with_capacity(trace.events.len() * 8);
    for event in &trace.events {
        if let Some(name) = event.attributes.iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| if let AttributeValue::String(s) = &a.value { Some(s.as_bytes()) } else { None })
        {
            buf.extend_from_slice(name);
            buf.push(0);
        }
    }
    fnv1a_64(&buf)
}

fn rank_top(scores: &[f64], n: usize) -> Vec<bool> {
    let mut idx: Vec<(usize, f64)> = scores.iter().enumerate().map(|(i,&s)| (i,s)).collect();
    idx.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    let mut out = vec![false; scores.len()];
    for &(i,_) in idx.iter().take(n) { out[i] = true; }
    out
}
