use dteam::autonomic::{AutonomicEvent, AutonomicKernel, DefaultKernel};
use std::thread;
use std::time::{Duration, SystemTime};
use log::{debug, info, warn};

fn main() {
    env_logger::init();
    let mut kernel = DefaultKernel::new();
    info!("🚀 Starting dteam Autonomic Runner...");
    debug!("Initial System State: {}\n", kernel.infer());

    let simulated_events = vec![
        ("sensor_alpha", "Trace packet received (ID: 1024)"),
        ("compliance_monitor", "Minor structural deviation detected"),
        (
            "adversary_aalst",
            "Direct structural repair triggered: Unsound Petri Net detected!",
        ),
        (
            "throughput_sensor",
            "Boutleneck detected in 'Approve' phase",
        ),
        ("sensor_beta", "High-frequency activity burst"),
    ];

    for (source, payload) in simulated_events {
        let event = AutonomicEvent {
            source: source.to_string(),
            payload: payload.to_string(),
            timestamp: SystemTime::now(),
        };

        info!("📥 Processing event: {}", event);

        let state = kernel.infer();
        let actions = kernel.propose(&state);

        let mut results = Vec::new();
        for action in actions {
            let accepted = kernel.accept(&action, &state);
            let status = if accepted {
                "✅ ACCEPTED"
            } else {
                "❌ REJECTED"
            };
            info!("  Action: {} -> {}", action.parameters, status);

            if accepted {
                results.push(kernel.execute(action));
            }
        }

        if results.is_empty() {
            warn!("  ℹ️  No actions were executed for event from {}.", event.source);
        } else {
            for res in &results {
                info!("  {}", kernel.manifest(res));
            }

            // Adapt with a small penalty to simulate operational decay
            debug!("Applying simulated operational decay penalty.");
            kernel.adapt(dteam::autonomic::AutonomicFeedback {
                reward: -0.5,
                human_override: false,
                side_effects: vec![],
            });
        }

        debug!("📊 Current State: {}\n", kernel.infer());

        // Simulate a small processing delay
        thread::sleep(Duration::from_millis(500));
    }

    info!("🏁 Autonomic Runner sequence complete.");
}
