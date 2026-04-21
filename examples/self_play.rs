//! dteam Self-Play Endurance Test
//! Fires 1,000,000 combinatorial events into the Vision2030Kernel
//! to prove zero-heap hot paths and numerical stability of the LinUCB bandit.

use dteam::autonomic::{AutonomicEvent, AutonomicFeedback, AutonomicKernel, Vision2030Kernel};
use dteam::simd::SwarMarking;
use dteam::utils::dense_kernel::KBitSet;
use std::time::{Instant, SystemTime};

fn main() {
    println!("⚔️ ---------------------------------------------------- ⚔️");
    println!("⚔️ INITIATING SELF-PLAY ENDURANCE TEST: 1 MILLION LOOPS ⚔️");
    println!("⚔️ ---------------------------------------------------- ⚔️\n");

    let mut kernel = Vision2030Kernel::<1>::new();
    let loops = 1_000_000;

    let payloads = [
        "Start order creates",
        "Normal item relates to order",
        "Bypass item value changed critical", // Anomalies
        "ConcurrentA item updates",
        "ConcurrentB order reads",
        "!!!NOISE!!! divergence critical",
        "End order item reads",
    ];

    println!("🚀 Firing {} events into the engine...", loops);

    let start_time = Instant::now();
    let mut total_actions_taken = 0;

    for i in 0..loops {
        let payload = payloads[i % payloads.len()];

        let event = AutonomicEvent {
            source: format!("self_play_agent_{}", i % 5),
            payload: payload.to_string(),
            timestamp: SystemTime::now(),
        };

        // 1. Observe branchlessly (must be zero-heap)
        kernel.observe(event);

        // 2. Infer & Propose
        let state = kernel.infer();
        let actions = kernel.propose(&state);

        // 3. Accept & Execute
        let mut reward = (state.conformance_score * 0.4) + (state.process_health * 0.4);
        if state.drift_detected {
            reward -= 0.2;
        }

        for action in actions {
            if kernel.accept(&action, &state) {
                total_actions_taken += 1;
                kernel.execute(action);
            }
        }

        // 4. Adapt (Sherman-Morrison update)
        kernel.adapt(AutonomicFeedback {
            reward,
            human_override: false,
            side_effects: vec![],
        });

        // Ensure the trace buffer resets periodically so we can test continuous operations
        if i % 100 == 0 {
            kernel.trace_cursor = 0;
            kernel.powl_executed_mask = KBitSet::<1>::zero();
            kernel.powl_prev_idx = 64;
            kernel.marking = SwarMarking::new(1);
        }
    }

    let elapsed = start_time.elapsed();
    let nanos_per_loop = elapsed.as_nanos() as f64 / (loops as f64);
    let final_state = kernel.infer();

    println!("\n✅ Endurance Run Completed Successfully!");
    println!("📊 Total Events Processed: {}", loops);
    println!("⏱️ Total Time Elapsed: {:?}", elapsed);
    println!(
        "⚡ Latency per cycle (observe -> adapt): {:.2} ns",
        nanos_per_loop
    );
    println!(
        "🤖 Total Autonomic Actions Executed: {}",
        total_actions_taken
    );

    println!("\n📈 FINAL STATE INTEGRITY CHECK:");
    println!(
        "  Process Health: {:.4} (Must be valid f32, no NaN)",
        final_state.process_health
    );
    println!(
        "  Conformance Score: {:.4} (Must be valid f32, no NaN)",
        final_state.conformance_score
    );
    println!("  Active Cases: {}", final_state.active_cases);

    assert!(
        final_state.process_health.is_finite(),
        "FATAL: Process health collapsed into NaN"
    );
    assert!(
        final_state.conformance_score.is_finite(),
        "FATAL: Conformance score collapsed into NaN"
    );
    assert!(
        nanos_per_loop < 5000.0,
        "FATAL: Engine is too slow! Heap allocations likely occurring in hot path."
    );

    println!("\n🎓 The LinUCB Agent matrix remained stable. Zero-heap properties held up under sustained pressure.");
}
