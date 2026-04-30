//! End-to-end Vision 2030 Kernel benchmarks.
//!
//! Measures complete autonomic cycles (observe → infer → propose → accept → execute → adapt)
//! to establish wall-clock latency for the paper's §7 claim: "2–50 µs per cycle".

use divan::{black_box, Bencher};
use dteam::autonomic::{AutonomicEvent, AutonomicFeedback, AutonomicKernel, Vision2030Kernel};
use std::time::SystemTime;

fn main() {
    divan::main();
}

#[divan::bench]
fn run_cycle_k64_single_event(bencher: Bencher) {
    let event = AutonomicEvent {
        source: "bench".to_string(),
        payload: "Normal".to_string(),
        timestamp: SystemTime::now(),
    };

    bencher.bench_local(|| {
        let mut kernel = Vision2030Kernel::<1>::new();
        // Single cycle: observe → infer → propose → accept → execute → adapt
        kernel.observe(black_box(event.clone()));
        let state = kernel.infer();
        let actions = kernel.propose(black_box(&state));
        if let Some(action) = actions.first() {
            if kernel.accept(action, &state) {
                let _ = kernel.execute(black_box(action.clone()));
            }
        }
        let feedback = AutonomicFeedback {
            reward: 0.5,
            human_override: false,
            side_effects: vec![],
        };
        kernel.adapt(feedback);
    });
}

#[divan::bench]
fn run_cycle_k64_six_event_epoch(bencher: Bencher) {
    let events = vec![
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Start".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Normal".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Normal".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Bypass".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "End".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Normal".to_string(),
            timestamp: SystemTime::now(),
        },
    ];

    bencher.bench_local(|| {
        let mut kernel = Vision2030Kernel::<1>::new();
        for event in events.iter() {
            kernel.observe(black_box(event.clone()));
            let state = kernel.infer();
            let actions = kernel.propose(black_box(&state));
            if let Some(action) = actions.first() {
                if kernel.accept(action, &state) {
                    let _ = kernel.execute(black_box(action.clone()));
                }
            }
            let feedback = AutonomicFeedback {
                reward: 0.5,
                human_override: false,
                side_effects: vec![],
            };
            kernel.adapt(feedback);
        }
    });
}

#[divan::bench]
fn run_cycle_k64_drift_recovery(bencher: Bencher) {
    let events = vec![
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Start".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Bypass".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "Bypass".to_string(),
            timestamp: SystemTime::now(),
        },
        AutonomicEvent {
            source: "bench".to_string(),
            payload: "End".to_string(),
            timestamp: SystemTime::now(),
        },
    ];

    bencher.bench_local(|| {
        let mut kernel = Vision2030Kernel::<1>::new();
        for event in events.iter() {
            kernel.observe(black_box(event.clone()));
            let state = kernel.infer();
            let actions = kernel.propose(black_box(&state));
            if let Some(action) = actions.first() {
                if kernel.accept(action, &state) {
                    let _ = kernel.execute(black_box(action.clone()));
                }
            }
            let feedback = AutonomicFeedback {
                reward: if state.drift_detected { 0.8 } else { 0.2 },
                human_override: false,
                side_effects: vec![],
            };
            kernel.adapt(feedback);
        }
    });
}
