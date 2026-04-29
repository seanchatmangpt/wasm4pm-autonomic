//! Latency benchmarks for the five symbolic+learned pairs in §5 of
//! `docs/COMPILED_COGNITION.md`.
//!
//! These replace the previously-fabricated nanosecond table; nanosecond figures
//! are hardware-dependent and the harness is the authoritative source.
//!
//! Run with: `cargo bench --bench symbolic_learned_bench`

use divan::{black_box, Bencher};

use dteam::ml::eliza::{keyword_bit, kw, turn_fast, DOCTOR};
use dteam::ml::hearsay::{run, Blackboard, Hypothesis, ACOUSTIC, DEFAULT_KS};
use dteam::ml::mycin::{infer_fast, RULES as MYCIN_RULES};
use dteam::ml::shrdlu::{initial_state, plan_cmd, Cmd};
use dteam::ml::strips::{
    plan_default, ARM_EMPTY, CLEAR_A, INITIAL_STATE, ON_A_B, ON_B_C, ON_TABLE_C,
};

fn main() {
    divan::main();
}

// =============================================================================
// ELIZA — Intent classification
// =============================================================================

#[divan::bench]
fn bench_eliza_turn_fast(bencher: Bencher) {
    let input = keyword_bit(kw::DREAM) | keyword_bit(kw::MOTHER);
    bencher.bench(|| turn_fast(black_box(input), black_box(&DOCTOR)));
}

// =============================================================================
// MYCIN — Diagnosis
// =============================================================================

#[divan::bench]
fn bench_mycin_infer_fast(bencher: Bencher) {
    // Representative bacteremia fact mask
    let facts: u64 = 0b0000_0000_0000_0000_0000_0001_0011_1011;
    bencher.bench(|| infer_fast(black_box(facts), black_box(&MYCIN_RULES)));
}

// =============================================================================
// STRIPS — Planning feasibility
// =============================================================================

#[divan::bench]
fn bench_strips_plan_default(bencher: Bencher) {
    // Goal: stack A on B on C, with arm empty. Reachable from default initial.
    let goal = ON_A_B | ON_B_C | CLEAR_A | ARM_EMPTY | ON_TABLE_C;
    bencher.bench(|| plan_default(black_box(INITIAL_STATE), black_box(goal)));
}

// =============================================================================
// SHRDLU — Spatial validation
// =============================================================================

#[divan::bench]
fn bench_shrdlu_plan_cmd(bencher: Bencher) {
    let state = initial_state();
    let cmd = Cmd::PickUp(0);
    bencher.bench(|| plan_cmd(black_box(state), black_box(cmd)));
}

// =============================================================================
// Hearsay-II — Multi-source signal fusion
// =============================================================================

#[divan::bench]
fn bench_hearsay_run(bencher: Bencher) {
    bencher
        .with_inputs(|| {
            let mut bb = Blackboard::new();
            bb.post(Hypothesis::new(ACOUSTIC, 0xAAAA_5555_AAAA_5555, 0.9, 0, 10));
            bb.post(Hypothesis::new(
                ACOUSTIC,
                0x5555_AAAA_5555_AAAA,
                0.85,
                5,
                15,
            ));
            bb.post(Hypothesis::new(
                ACOUSTIC,
                0xFFFF_0000_FFFF_0000,
                0.95,
                10,
                20,
            ));
            bb
        })
        .bench_local_refs(|bb| run(bb, &DEFAULT_KS, 16));
}
