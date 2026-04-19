use dteam::autonomic::{DefaultKernel, AutonomicKernel, AutonomicEvent};
use std::time::{SystemTime, Duration};
use std::thread;

fn main() {
    let mut kernel = DefaultKernel::new();
    println!("🚀 Starting dteam Autonomic Runner...");
    println!("Initial System State: {}\n", kernel.infer());

    let simulated_events = vec![
        ("sensor_alpha", "Trace packet received (ID: 1024)"),
        ("compliance_monitor", "Minor structural deviation detected"),
        ("adversary_aalst", "Direct structural repair triggered: Unsound Petri Net detected!"),
        ("throughput_sensor", "Boutleneck detected in 'Approve' phase"),
        ("sensor_beta", "High-frequency activity burst"),
    ];

    for (source, payload) in simulated_events {
        let event = AutonomicEvent {
            source: source.to_string(),
            payload: payload.to_string(),
            timestamp: SystemTime::now(),
        };

        println!("📥 {}", event);
        
        let state = kernel.infer();
        let actions = kernel.propose(&state);
        
        let mut results = Vec::new();
        for action in actions {
            let accepted = kernel.accept(&action, &state);
            let status = if accepted { "✅ ACCEPTED" } else { "❌ REJECTED" };
            println!("  {} -> {}", action, status);
            
            if accepted {
                results.push(kernel.execute(action));
            }
        }
        
        if results.is_empty() {
            println!("  ℹ️  No actions were executed.");
        } else {
            for res in &results {
                println!("  {}", kernel.manifest(res));
            }
            
            // Adapt with a small penalty to simulate operational decay
            kernel.adapt(dteam::autonomic::AutonomicFeedback {
                reward: -0.5,
                human_override: false,
                side_effects: vec![],
            });
        }
        
        println!("📊 Current State: {}\n", kernel.infer());
        
        // Simulate a small processing delay
        thread::sleep(Duration::from_millis(500));
    }

    println!("🏁 Autonomic Runner sequence complete.");
}
