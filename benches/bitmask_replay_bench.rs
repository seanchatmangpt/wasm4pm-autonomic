//! Latency benchmarks for u64-bitmask Petri-net token replay
//! (`src/conformance/bitmask_replay.rs`).
//!
//! Substantiates §7 of `docs/COMPILED_COGNITION.md` ("conformance is sub-µs").
//! `NetBitmask64` requires ≤64 places, so we sweep K=8/16/32/64.
//!
//! Run with: `cargo bench --bench bitmask_replay_bench`

use divan::{black_box, Bencher};

use dteam::conformance::bitmask_replay::{
    classify_exact, in_language, replay_log, replay_trace, NetBitmask64,
};
use dteam::models::petri_net::{Arc, PetriNet, Place, Transition};
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use dteam::utils::dense_kernel::{fnv1a_64, PackedKeyTable};

fn main() {
    divan::main();
}

/// Build a linear Petri net with `k` places (p0 -t0-> p1 -t1-> p2 -...-> pk).
/// k must satisfy 2 ≤ k ≤ 64.
fn build_linear_net(k: usize) -> PetriNet {
    assert!((2..=64).contains(&k));
    let places: Vec<Place> = (0..k)
        .map(|i| Place {
            id: format!("p{i}"),
        })
        .collect();
    let transitions: Vec<Transition> = (0..k - 1)
        .map(|i| Transition {
            id: format!("t{i}"),
            label: format!("a{i}"),
            is_invisible: Some(false),
        })
        .collect();
    let mut arcs = Vec::with_capacity(2 * (k - 1));
    for i in 0..k - 1 {
        arcs.push(Arc {
            from: format!("p{i}"),
            to: format!("t{i}"),
            weight: None,
        });
        arcs.push(Arc {
            from: format!("t{i}"),
            to: format!("p{}", i + 1),
            weight: None,
        });
    }
    let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
    im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
    let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
    let last = format!("p{}", k - 1);
    fm.insert(fnv1a_64(last.as_bytes()), last, 1);
    PetriNet {
        places,
        transitions,
        arcs,
        initial_marking: im,
        final_markings: vec![fm],
        ..Default::default()
    }
}

fn make_trace(k: usize) -> Trace {
    let events: Vec<Event> = (0..k - 1)
        .map(|i| Event {
            attributes: vec![Attribute {
                key: "concept:name".into(),
                value: AttributeValue::String(format!("a{i}")),
            }],
        })
        .collect();
    Trace {
        id: "t".into(),
        attributes: vec![],
        events,
    }
}

fn make_log(k: usize, n: usize) -> EventLog {
    let mut log = EventLog::new();
    for _ in 0..n {
        log.traces.push(make_trace(k));
    }
    log
}

// =============================================================================
// replay_trace — single-trace token replay
// =============================================================================

#[divan::bench(args = [8, 16, 32, 64])]
fn bench_replay_trace(bencher: Bencher, k: usize) {
    let net = build_linear_net(k);
    let bm = NetBitmask64::from_petri_net(&net);
    let trace = make_trace(k);
    bencher.bench(|| replay_trace(black_box(&bm), black_box(&trace)));
}

// =============================================================================
// replay_log — 1 000-trace log
// =============================================================================

#[divan::bench(args = [8, 16, 32, 64])]
fn bench_replay_log_1k(bencher: Bencher, k: usize) {
    let net = build_linear_net(k);
    let bm = NetBitmask64::from_petri_net(&net);
    let log = make_log(k, 1_000);
    bencher.bench(|| replay_log(black_box(&bm), black_box(&log)));
}

// =============================================================================
// in_language — exact acceptance check
// =============================================================================

#[divan::bench(args = [8, 16, 32, 64])]
fn bench_in_language(bencher: Bencher, k: usize) {
    let net = build_linear_net(k);
    let bm = NetBitmask64::from_petri_net(&net);
    let trace = make_trace(k);
    bencher.bench(|| in_language(black_box(&bm), black_box(&trace)));
}

// =============================================================================
// classify_exact — full log classification
// =============================================================================

#[divan::bench(args = [8, 16, 32, 64])]
fn bench_classify_exact(bencher: Bencher, k: usize) {
    let net = build_linear_net(k);
    let bm = NetBitmask64::from_petri_net(&net);
    let log = make_log(k, 100);
    bencher.bench(|| classify_exact(black_box(&bm), black_box(&log), 100));
}
