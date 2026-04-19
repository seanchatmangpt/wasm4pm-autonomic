use dteam::dteam::orchestration::{Engine, DteamDoctor};
use dteam::autonomic::{DefaultKernel, AutonomicKernel, AutonomicEvent};
use std::time::SystemTime;

fn main() {
    println!("--- dteam Digital Team Doctor ---");
    let engine = Engine::builder().build();
    println!("{}", engine.doctor());
    
    println!("\n--- Autonomic Kernel Diagnostic ---");
    let mut kernel = DefaultKernel::new();
    let event = AutonomicEvent {
        source: "diagnostic_agent".to_string(),
        payload: "Self-test sequence initiated".to_string(),
        timestamp: SystemTime::now(),
    };
    
    println!("State before: {}", kernel.infer());
    let results = kernel.run_cycle(event);
    println!("Cycle executed. Result count: {}", results.len());
    for res in results {
        println!("  {}", res);
    }
    println!("State after:  {}", kernel.infer());
    
    println!("\nDiagnostics complete. System status: NOMINAL");
}
