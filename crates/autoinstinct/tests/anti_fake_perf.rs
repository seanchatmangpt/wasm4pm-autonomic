//! Kill Zone 6 — Performance Honesty Gauntlet.
//!
//! Proves the hot path is *physically* honest: `decide()` and
//! `select_instinct_v0()` allocate zero bytes across **every** scenario
//! in [`autoinstinct::causal_harness::canonical_scenarios`] and across
//! every canonical response class.
//!
//! The gauntlet uses a process-wide allocation-counting allocator. Each
//! integration test binary gets its own `#[global_allocator]`, so this
//! does not collide with `crates/ccog/tests/gauntlet.rs`.
//!
//! Cross-zone invariant: the same scenarios that prove causal dependency
//! (Kill Zone 2) here prove zero-heap purity (Kill Zone 6). A fake hot
//! path either fails alloc=0 or fails causal change — it can never pass
//! both.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use autoinstinct::causal_harness::{
    build_inputs, canonical_scenarios, perturb, CausalScenario,
};
use ccog::bark_artifact::decide;
use ccog::compiled::CompiledFieldSnapshot;
use ccog::instinct::select_instinct_v0;
use ccog::multimodal::{ContextBundle, PostureBundle};

// =============================================================================
// Counting allocator — thread-local so parallel tests don't contaminate.
// =============================================================================

struct CountingAlloc;

thread_local! {
    static TL_BYTES: Cell<u64> = const { Cell::new(0) };
    static TL_COUNT: Cell<u64> = const { Cell::new(0) };
    static TL_ENABLED: Cell<bool> = const { Cell::new(false) };
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = TL_ENABLED.try_with(|e| {
            if e.get() {
                TL_BYTES.with(|b| b.set(b.get() + layout.size() as u64));
                TL_COUNT.with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn measure<R>(f: impl FnOnce() -> R) -> (R, u64, u64) {
    TL_BYTES.with(|b| b.set(0));
    TL_COUNT.with(|c| c.set(0));
    TL_ENABLED.with(|e| e.set(true));
    let r = f();
    TL_ENABLED.with(|e| e.set(false));
    (r, TL_BYTES.with(|b| b.get()), TL_COUNT.with(|c| c.get()))
}

// Build snapshot + posture + ctx ONCE outside the measurement so we
// only weigh the hot path itself.
fn closed_surface(s: &CausalScenario) -> (CompiledFieldSnapshot, PostureBundle, ContextBundle) {
    let (field, posture, ctx) = build_inputs(s);
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    (snap, posture, ctx)
}

// Force initialization of OnceLock statics inside compute_present_mask before measurement.
fn force_init_statics() {
    let (field, _, _) = build_inputs(&canonical_scenarios()[0]);
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let _ = decide(&snap);
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn anti_fake_perf_control_allocation_is_detected() {
    // Positive control: proves CountingAlloc actually counts allocations.
    // If this test fails, all zero-alloc assertions are vacuous — the allocator
    // is not measuring anything.
    let (_, bytes, count) = measure(|| {
        let v: Vec<u8> = vec![1u8, 2, 3, 4, 5];
        std::hint::black_box(v)
    });
    assert!(
        count >= 1,
        "CountingAlloc did NOT detect a deliberate Vec allocation \
         (bytes={bytes}, count={count}) — all zero-alloc assertions are vacuous"
    );
    assert!(bytes >= 5, "bytes={bytes} is unexpectedly small for a 5-byte Vec");
}

#[test]
fn performance_decide_zero_alloc_generated_snapshots() {
    force_init_statics();
    for s in canonical_scenarios() {
        let (snap, posture, ctx) = closed_surface(&s);
        // Warm up to dodge any first-call lazy init.
        let _ = decide(&snap);
        let _ = decide(&snap);
        let (decision, d_bytes, d_count) = measure(|| decide(&snap));
        let _ = select_instinct_v0(&snap, &posture, &ctx);
        let (_, s_bytes, s_count) = measure(|| select_instinct_v0(&snap, &posture, &ctx));
        println!(
            "scenario={} decide_allocations={} decide_bytes={} select_allocations={} select_bytes={}",
            s.name, d_count, d_bytes, s_count, s_bytes
        );
        assert_eq!(
            d_bytes, 0,
            "scenario `{}`: decide() allocated {d_bytes} bytes across {d_count} allocations",
            s.name
        );
        assert_eq!(d_count, 0, "scenario `{}`: decide() ran {d_count} allocations", s.name);
        assert_eq!(s_bytes, 0, "scenario `{}`: select_instinct_v0 alloc bytes={s_bytes}", s.name);
        assert_eq!(s_count, 0);
        let _ = decision;
    }
}

#[test]
fn performance_select_instinct_zero_alloc_all_response_classes() {
    force_init_statics();
    for s in canonical_scenarios() {
        let (snap, posture, ctx) = closed_surface(&s);
        // Warm up.
        let _ = select_instinct_v0(&snap, &posture, &ctx);
        let _ = select_instinct_v0(&snap, &posture, &ctx);
        let (resp, bytes, count) =
            measure(|| select_instinct_v0(&snap, &posture, &ctx));
        assert_eq!(
            bytes, 0,
            "scenario `{}` ({:?}): select_instinct_v0 allocated {bytes} bytes",
            s.name, resp
        );
        assert_eq!(count, 0, "scenario `{}`: select_instinct_v0 ran {count} allocations", s.name);
    }
}

#[test]
fn performance_decide_does_not_materialize_or_seal() {
    force_init_statics();
    // If decide() materialized (built Construct8) or sealed (built a
    // receipt) it would allocate bytes — deltas and BLAKE3 hashers both
    // touch the heap. Zero alloc across every scenario is the proof.
    for s in canonical_scenarios() {
        let (snap, _, _) = closed_surface(&s);
        let _ = decide(&snap);
        let (_, bytes, _) = measure(|| {
            for _ in 0..16 {
                let _ = decide(&snap);
            }
        });
        assert_eq!(
            bytes, 0,
            "scenario `{}`: decide() allocated under repeated calls — \
             likely materializing or sealing on the hot path",
            s.name
        );
    }
}

#[test]
fn performance_perturbed_decide_remains_zero_alloc() {
    force_init_statics();
    // Cross-zone: under each load-bearing perturbation, decide() must
    // remain alloc-free. A regression that allocates only on certain
    // input shapes would slip past a single-fixture check.
    for s in canonical_scenarios() {
        for (pert, _) in &s.perturbations {
            let (field, _, _) = perturb(&s, pert);
            let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
            let _ = decide(&snap);
            let (_, bytes, count) = measure(|| decide(&snap));
            assert_eq!(
                bytes, 0,
                "scenario `{}` under perturbation {:?}: decide() allocated {bytes} bytes",
                s.name, pert
            );
            assert_eq!(count, 0);
        }
    }
}

#[test]
fn anti_fake_decide_is_zero_heap_and_input_dependent() {
    force_init_statics();
    // The cross-zone invariant. For each scenario:
    //   1. decide()/select_instinct_v0() under closed surface allocate 0.
    //   2. Under perturbation, response changes.
    //   3. Perturbed decide() still allocates 0.
    // A fake hot path either fails (1)/(3) (impurity) or fails (2)
    // (coincidence). It cannot pass both.
    let mut input_changed_count = 0;
    for s in canonical_scenarios() {
        let (baseline, alloc_before) = {
            let (f, p, c) = build_inputs(&s);
            let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
            let _ = select_instinct_v0(&snap, &p, &c); // warm
            let (resp, bytes, _) = measure(|| select_instinct_v0(&snap, &p, &c));
            assert_eq!(bytes, 0, "baseline alloc != 0 for `{}`", s.name);
            (resp, bytes)
        };
        for (pert, _expected) in &s.perturbations {
            let (f, p, c) = perturb(&s, pert);
            let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
            let _ = select_instinct_v0(&snap, &p, &c); // warm
            let (after, alloc_after, count) =
                measure(|| select_instinct_v0(&snap, &p, &c));
            let changed = after != baseline;
            println!(
                "cross_zone scenario={} alloc_before={} alloc_after={} count={} changed={}",
                s.name, alloc_before, alloc_after, count, changed
            );
            assert_eq!(
                alloc_after, 0,
                "scenario `{}` perturbation {:?}: alloc bytes={alloc_after} count={count}",
                s.name, pert
            );
            assert!(
                changed,
                "scenario `{}` perturbation {:?}: response did not change \
                 (still {:?}) — input is not load-bearing",
                s.name, pert, after
            );
            input_changed_count += 1;
        }
    }
    assert!(
        input_changed_count >= 7,
        "expected ≥7 perturbation cases across 7 scenarios; saw {input_changed_count}"
    );
}
