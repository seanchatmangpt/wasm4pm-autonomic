use divan::black_box;
use dteam::autonomic::vision_2030_kernel::Vision2030Kernel;
use dteam::autonomic::{
    ActionRisk, ActionType, AutonomicAction, AutonomicEvent, AutonomicFeedback, AutonomicKernel,
};
use std::time::SystemTime;

fn main() {
    divan::main();
}

/// Benchmark a single MAPE-K cycle: observe -> infer -> propose -> accept -> execute -> adapt
#[divan::bench]
fn bench_vision2030_kernel_single_cycle() {
    let mut kernel = black_box(Vision2030Kernel::<4>::new());

    // Create a representative autonomic event
    let event = AutonomicEvent {
        source: "order_system".to_string(),
        payload: "start matched normal order item creates obj1".to_string(),
        timestamp: SystemTime::now(),
    };

    divan::black_box({
        // Observe phase: ingest event
        kernel.observe(event);

        // Infer phase: read state
        let state = kernel.infer();

        // Propose phase: generate action candidates
        let actions = kernel.propose(&state);

        if !actions.is_empty() {
            let action = &actions[0];

            // Accept phase: evaluate action acceptability
            let accepted = kernel.accept(action, &state);

            // Execute phase: run the action
            if accepted {
                let result = kernel.execute(black_box(action.clone()));

                // Adapt phase: update bandit and state
                let feedback = AutonomicFeedback {
                    reward: 0.1f32,
                    human_override: false,
                    side_effects: vec![],
                };
                kernel.adapt(feedback);

                black_box(result);
            }
        }
    });
}

/// Benchmark 100 sequential MAPE-K cycles to measure learning loop throughput
#[divan::bench]
fn bench_vision2030_kernel_100_cycles() {
    let mut kernel = black_box(Vision2030Kernel::<4>::new());

    divan::black_box({
        for cycle in 0..100 {
            // Vary the event payload to simulate realistic event stream
            let payload = match cycle % 6 {
                0 => "start transition",
                1 => "normal processing matched",
                2 => "bypass skip violation",
                3 => "end finish limit",
                4 => "concurrent activity A obj1",
                _ => "divergence critical item obj2 creates",
            };

            let event = AutonomicEvent {
                source: format!("system_{}", cycle),
                payload: payload.to_string(),
                timestamp: SystemTime::now(),
            };

            // Full MAPE-K cycle
            kernel.observe(event);
            let state = kernel.infer();
            let actions = kernel.propose(&state);

            if !actions.is_empty() {
                let action = &actions[0];
                if kernel.accept(action, &state) {
                    let result = kernel.execute(black_box(action.clone()));
                    let feedback = AutonomicFeedback {
                        reward: if cycle % 3 == 0 { 0.2 } else { -0.1 },
                        human_override: cycle % 50 == 0,
                        side_effects: vec![],
                    };
                    kernel.adapt(feedback);
                    black_box(result);
                }
            }
        }
    });
}

/// Benchmark manifest hash extraction from AutonomicResult
/// Tests the cost of fnv1a_64 hashing on action parameters
#[divan::bench]
fn bench_manifest_hash_extraction() {
    let mut kernel = black_box(Vision2030Kernel::<4>::new());

    divan::black_box({
        let action = AutonomicAction::new(
            102u64,
            ActionType::Repair,
            ActionRisk::Medium,
            "Axiomatic structural repair with full trace reset",
        );

        // Execute produces a result with manifest_hash
        let result = kernel.execute(action);

        // Extract manifest (calls fnv1a_64 internally)
        let manifest = kernel.manifest(&result);

        black_box(manifest)
    });
}

/// Benchmark the full cycle feedback roundtrip:
/// execute action -> manifest extraction -> adapt (with bandit update)
#[divan::bench]
fn bench_cycle_feedback_roundtrip() {
    let mut kernel = black_box(Vision2030Kernel::<4>::new());

    divan::black_box({
        // Prime the kernel with a few observations
        for i in 0..5 {
            let event = AutonomicEvent {
                source: "prep".to_string(),
                payload: format!("normal item obj{}", i),
                timestamp: SystemTime::now(),
            };
            kernel.observe(event);
        }

        // Establish a state by running infer
        let state = kernel.infer();
        let pre_action_conformance = state.conformance_score;

        // Propose and execute
        let actions = kernel.propose(&state);
        if !actions.is_empty() {
            let action = &actions[0];
            if kernel.accept(action, &state) {
                let result = kernel.execute(black_box(action.clone()));

                // Extract manifest and verify
                let _manifest = kernel.manifest(&result);

                // Simulate feedback roundtrip with reward differential
                let post_state = kernel.infer();
                let reward = post_state.conformance_score - pre_action_conformance;

                let feedback = AutonomicFeedback {
                    reward: reward.clamp(-1.0, 1.0),
                    human_override: false,
                    side_effects: vec!["trace_reset".to_string()],
                };

                // Adapt phase completes the roundtrip (bandit update + state decay)
                kernel.adapt(feedback);

                black_box((result, _manifest));
            }
        }
    });
}
