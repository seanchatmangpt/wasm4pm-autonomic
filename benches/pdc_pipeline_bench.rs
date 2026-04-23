use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dteam::conformance::bitmask_replay::{classify_exact, replay_log, NetBitmask64};
use dteam::ml::hdc;
use dteam::ml::pdc_combinator::run_combinator;
use dteam::ml::pdc_features::{extract_log_features, extract_log_features_with_vocab};
use dteam::ml::pdc_supervised::{run_supervised, run_supervised_transfer};
use dteam::models::petri_net::{Arc, PetriNet, Place, Transition};
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use dteam::utils::dense_kernel::{fnv1a_64, PackedKeyTable};
use std::time::Duration;

// ── Synthetic fixtures ────────────────────────────────────────────────────────

const ACTS_SHORT: &[&str] = &["A", "B", "C", "D", "E"];
const ACTS_FULL: &[&str] = &["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

/// Linear sequence net: p0 -A-> p1 -B-> ... -E-> p5
fn sequence_net(activities: &[&str]) -> PetriNet {
    let n = activities.len();
    let places: Vec<Place> = (0..=n)
        .map(|i| Place {
            id: format!("p{i}"),
        })
        .collect();
    let mut transitions = Vec::new();
    let mut arcs = Vec::new();

    for (i, act) in activities.iter().enumerate() {
        let tid = format!("t_{act}");
        transitions.push(Transition {
            id: tid.clone(),
            label: act.to_string(),
            is_invisible: Some(false),
        });
        arcs.push(Arc {
            from: format!("p{i}"),
            to: tid.clone(),
            weight: None,
        });
        arcs.push(Arc {
            from: tid,
            to: format!("p{}", i + 1),
            weight: None,
        });
    }

    let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
    im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
    let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
    let end = format!("p{n}");
    fm.insert(fnv1a_64(end.as_bytes()), end.clone(), 1);

    PetriNet {
        places,
        transitions,
        arcs,
        initial_marking: im,
        final_markings: vec![fm],
        ..Default::default()
    }
}

fn make_event(act: &str) -> Event {
    Event {
        attributes: vec![Attribute {
            key: "concept:name".into(),
            value: AttributeValue::String(act.into()),
        }],
    }
}

/// 1000-trace log mixing conforming (25%), partial (25%), extended (25%), shuffled (25%).
fn synthetic_log(n: usize, activities: &[&str]) -> EventLog {
    let mut traces = Vec::with_capacity(n);
    for i in 0..n {
        let mut t = Trace {
            id: format!("t{i}"),
            events: Vec::new(),
            attributes: Vec::new(),
        };
        match i % 4 {
            0 => {
                for &a in activities {
                    t.events.push(make_event(a));
                }
            }
            1 => {
                for (j, &a) in activities.iter().enumerate() {
                    if (i + j) % 3 != 0 {
                        t.events.push(make_event(a));
                    }
                }
            }
            2 => {
                for &a in activities {
                    t.events.push(make_event(a));
                }
                t.events.push(make_event(activities[i % activities.len()]));
            }
            _ => {
                let off = i % activities.len();
                for j in 0..activities.len() {
                    t.events
                        .push(make_event(activities[(j + off) % activities.len()]));
                }
            }
        }
        traces.push(t);
    }
    EventLog {
        traces,
        attributes: Vec::new(),
    }
}

/// Extract activity sequences for HDC benchmarks.
fn log_to_seqs(log: &EventLog) -> Vec<Vec<String>> {
    log.traces
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
        .collect()
}

// ── Benchmarks ────────────────────────────────────────────────────────────────

/// Hot path 1: token replay over the full log.
/// This is called once per log per strategy — 96× in the PDC pipeline.
fn bench_replay_log(c: &mut Criterion) {
    let net = sequence_net(ACTS_SHORT);
    let bm = NetBitmask64::from_petri_net(&net);
    let log = synthetic_log(1000, ACTS_SHORT);

    let mut group = c.benchmark_group("PDC/replay");
    group.warm_up_time(Duration::from_millis(500));
    group.sample_size(50);
    group.bench_function("replay_log/1000_traces", |b| {
        b.iter(|| replay_log(black_box(&bm), black_box(&log)))
    });
    group.bench_function("classify_exact/1000_traces", |b| {
        b.iter(|| classify_exact(black_box(&bm), black_box(&log), black_box(500)))
    });
    group.finish();
}

/// Hot path 2: feature extraction — feeds all 10+ supervised classifiers.
fn bench_feature_extraction(c: &mut Criterion) {
    let net5 = sequence_net(ACTS_SHORT);
    let bm5 = NetBitmask64::from_petri_net(&net5);
    let log1k = synthetic_log(1000, ACTS_SHORT);

    let net10 = sequence_net(ACTS_FULL);
    let bm10 = NetBitmask64::from_petri_net(&net10);
    let log10 = synthetic_log(1000, ACTS_FULL);

    let vocab = dteam::ml::pdc_features::build_vocabulary(&log1k);
    let max_len = log1k
        .traces
        .iter()
        .map(|t| t.events.len())
        .max()
        .unwrap_or(1);

    let mut group = c.benchmark_group("PDC/features");
    group.warm_up_time(Duration::from_millis(500));
    group.sample_size(50);
    group.bench_function("extract_log_features/5_acts", |b| {
        b.iter(|| extract_log_features(black_box(&log1k), black_box(&bm5)))
    });
    group.bench_function("extract_log_features/10_acts", |b| {
        b.iter(|| extract_log_features(black_box(&log10), black_box(&bm10)))
    });
    group.bench_function("extract_log_features_with_vocab/1000_traces", |b| {
        b.iter(|| {
            extract_log_features_with_vocab(
                black_box(&log1k),
                black_box(&bm5),
                black_box(&vocab),
                black_box(max_len),
            )
        })
    });
    group.finish();
}

/// Hot path 3: 10-classifier supervised ensemble (transductive and transfer).
fn bench_supervised(c: &mut Criterion) {
    let net = sequence_net(ACTS_SHORT);
    let bm = NetBitmask64::from_petri_net(&net);
    let log = synthetic_log(1000, ACTS_SHORT);

    let (features, in_lang, _) = extract_log_features(&log, &bm);

    // Transfer: 40 labeled training samples (simulating the _11 file)
    let train_feats: Vec<Vec<f64>> = features.iter().take(40).cloned().collect();
    let train_labels: Vec<bool> = (0..40).map(|i| i < 20).collect();

    let mut group = c.benchmark_group("PDC/supervised");
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(15);
    group.bench_function("run_supervised/1000_traces", |b| {
        b.iter(|| run_supervised(black_box(&features), black_box(&in_lang)))
    });
    group.bench_function("run_supervised_transfer/40_train_1000_test", |b| {
        b.iter(|| {
            run_supervised_transfer(
                black_box(&train_feats),
                black_box(&train_labels),
                black_box(&features),
            )
        })
    });
    group.finish();
}

/// Hot path 4: HDC — unsupervised (fit+classify) and discriminative (fit_labeled+classify_labeled).
fn bench_hdc(c: &mut Criterion) {
    let log = synthetic_log(1000, ACTS_FULL);
    let seqs = log_to_seqs(&log);

    // 40 labeled sequences for discriminative HDC
    let labeled_seqs: Vec<Vec<String>> = seqs.iter().take(40).cloned().collect();
    let labels: Vec<bool> = (0..40).map(|i| i < 20).collect();
    let extra_seqs: Vec<Vec<String>> = seqs.iter().skip(40).take(960).cloned().collect();

    let mut group = c.benchmark_group("PDC/hdc");
    group.warm_up_time(Duration::from_millis(500));
    group.sample_size(50);
    group.bench_function("fit+classify/1000_traces", |b| {
        b.iter(|| {
            let clf = hdc::fit(black_box(&seqs));
            hdc::classify(black_box(&clf), black_box(&seqs), black_box(500))
        })
    });
    group.bench_function("fit_labeled+classify_labeled/40_train_1000_test", |b| {
        b.iter(|| {
            let clf = hdc::fit_labeled(
                black_box(&labeled_seqs),
                black_box(&labels),
                black_box(&extra_seqs),
            );
            hdc::classify_labeled(black_box(&clf), black_box(&seqs), black_box(500))
        })
    });
    group.finish();
}

/// Hot path 5: full run_combinator — baseline (pseudo-labels) vs labeled transfer.
/// Measures the real competition wall-clock and the speedup from the fix.
fn bench_combinator(c: &mut Criterion) {
    let net = sequence_net(ACTS_SHORT);
    let bm = NetBitmask64::from_petri_net(&net);

    // 40-trace labeled training log (simulates _11 file)
    let train_log = synthetic_log(40, ACTS_SHORT);
    let train_labels: Vec<bool> = (0..40).map(|i| i < 20).collect();

    let mut group = c.benchmark_group("PDC/combinator");
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(10);
    for n_traces in [100, 1000] {
        let log = synthetic_log(n_traces, ACTS_SHORT);

        // Baseline: pseudo-labels on 1000 traces (old path)
        group.bench_with_input(
            BenchmarkId::new("run_combinator/pseudo", n_traces),
            &n_traces,
            |b, _| {
                b.iter(|| {
                    run_combinator(
                        black_box(&log),
                        black_box(&bm),
                        black_box(n_traces / 2),
                        black_box(Some("pdc2025_010101")),
                        black_box(None),
                    )
                })
            },
        );

        // Fixed: real labels from 40 training traces
        group.bench_with_input(
            BenchmarkId::new("run_combinator/labeled", n_traces),
            &n_traces,
            |b, _| {
                b.iter(|| {
                    run_combinator(
                        black_box(&log),
                        black_box(&bm),
                        black_box(n_traces / 2),
                        black_box(Some("pdc2025_010101")),
                        black_box(Some((&train_log, train_labels.as_slice()))),
                    )
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_replay_log,
    bench_feature_extraction,
    bench_supervised,
    bench_hdc,
    bench_combinator,
);
criterion_main!(benches);
