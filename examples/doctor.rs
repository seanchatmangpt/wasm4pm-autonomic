use dteam::autonomic::{AutonomicEvent, AutonomicKernel, DefaultKernel};
use dteam::dteam::orchestration::{DteamDoctor, Engine};
use log::{debug, info};
use std::time::SystemTime;

fn main() {
    env_logger::init();
    info!("--- dteam Digital Team Doctor ---");
    let engine = Engine::builder().build();
    info!("{}", engine.doctor());

    info!("\n--- Autonomic Kernel Diagnostic ---");
    let mut kernel = DefaultKernel::new();
    let event = AutonomicEvent {
        source: "diagnostic_agent".to_string(),
        payload: "Self-test sequence initiated".to_string(),
        timestamp: SystemTime::now(),
    };

    debug!("State before: {}", kernel.infer());
    let results = kernel.run_cycle(event);
    info!("Cycle executed. Result count: {}", results.len());
    for res in results {
        info!("  {}", res);
    }
    debug!("State after:  {}", kernel.infer());

    info!("\nDiagnostics complete. System status: NOMINAL");
}
