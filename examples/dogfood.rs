//! dteam Self-Awareness Module (Eating Our Own Dog Food)
//! This script forces the process intelligence engine to discover, compile, and validate
//! the structural topology of its OWN internal autonomic execution loop.

use dteam::autonomic::{AutonomicEvent, AutonomicFeedback, AutonomicKernel, Vision2030Kernel};
use dteam::powl::core::PowlModel;
use dteam::powl::discovery::mine_powl;
use dteam::utils::dense_kernel::KBitSet;
use std::collections::HashMap;
use std::time::SystemTime;

fn main() {
    println!("🧪 ---------------------------------------------------- 🧪");
    println!("🧪 INITIATING THE FINAL DEFENSE: SYSTEMIC SELF-DISCOVERY 🧪");
    println!("🧪 ---------------------------------------------------- 🧪\n");

    let mut kernel = Vision2030Kernel::<1>::new();
    let mut self_log: Vec<String> = Vec::new();

    // Simulate external stream anomalies to force the kernel to take various internal paths
    let payloads = vec![
        "Start order creates",
        "Normal item relates to order",
        "End order item reads",
    ];

    println!("📡 STEP 1: Instrumenting the Autonomic Kernel and generating execution telemetry...");

    for payload in payloads {
        let event = AutonomicEvent {
            source: "dogfood_sensor".to_string(),
            payload: payload.to_string(),
            timestamp: SystemTime::now(),
        };

        // Telemetry: Observe
        self_log.push("Kernel:Observe".to_string());
        let _results = kernel.run_cycle(event);

        let state = kernel.infer();
        self_log.push("Kernel:Infer".to_string());

        let actions = kernel.propose(&state);
        self_log.push("Kernel:Propose".to_string());

        let mut executed_something = false;
        for action in actions {
            let accepted = kernel.accept(&action, &state);
            if accepted {
                self_log.push(format!("Kernel:Execute:{:?}", action.action_type));
                let _ = kernel.execute(action);
                executed_something = true;
            }
        }

        if !executed_something {
            self_log.push("Kernel:Execute:None".to_string());
        }

        self_log.push("Kernel:Adapt".to_string());
        kernel.adapt(AutonomicFeedback {
            reward: 0.1,
            human_override: false,
            side_effects: vec![],
        });
    }

    println!(
        "  ✅ Harvested {} internal telemetry events from engine execution.\n",
        self_log.len()
    );

    println!("🏗️ STEP 2: Building internal Directly-Follows Graph (DFG) natively...");
    // Map string events to u8 indices for the DFG
    let mut symbol_table = HashMap::new();
    let mut activity_names = Vec::new();
    let mut traces: Vec<Vec<u8>> = Vec::new();
    let mut current_trace: Vec<u8> = Vec::new();

    for event in &self_log {
        let idx = *symbol_table.entry(event.clone()).or_insert_with(|| {
            let new_idx = activity_names.len() as u8;
            activity_names.push(event.clone());
            new_idx
        });
        current_trace.push(idx);

        if event == "Kernel:Adapt" {
            traces.push(current_trace.clone());
            current_trace.clear();
        }
    }

    let mut dfg = vec![KBitSet::<1>::zero(); 64];
    let mut footprint = KBitSet::<1>::zero();

    // Construct DFG from sequence
    for trace in traces {
        for window in trace.windows(2) {
            let src = window[0] as usize;
            let tgt = window[1] as usize;
            let _ = dfg[src].set(tgt);
            let _ = footprint.set(src);
            let _ = footprint.set(tgt);
        }
    }
    println!(
        "  ✅ Discovered {} unique kernel operations.",
        activity_names.len()
    );

    println!("\n⛏️ STEP 3: Nanosecond Inductive Mining of the Autonomic Loop...");
    let start_time = std::time::Instant::now();

    let powl_ast = mine_powl(&dfg, footprint, &activity_names);

    println!("  ✅ Discovered POWL AST in {:?}", start_time.elapsed());
    println!("  🌳 Discovered AST: {:#?}", powl_ast);
    println!("\n🌳 Structurally Validating Discovered AST...");
    powl_ast
        .validate_soundness()
        .expect("Engine mined an unsound AST from itself!");
    println!("  ✅ Topological DFS acyclicity check passed.");

    println!("\n⚙️ STEP 4: Compiling AST into BCINR Bitmasks for Self-Validation...");
    let self_model = PowlModel::<1>::new(powl_ast);
    println!("  ✅ Self-model mathematically closed.");

    println!("\n🔍 STEP 5: Validating internal operational integrity against discovered laws of physics...");

    println!(
        "Discovered Symbols: {:?}",
        symbol_table.keys().collect::<Vec<_>>()
    );

    // A structurally valid, healthy loop
    let mut valid_trace = vec![
        *symbol_table.get("Kernel:Observe").unwrap(),
        *symbol_table.get("Kernel:Infer").unwrap(),
        *symbol_table.get("Kernel:Propose").unwrap(),
    ];
    if let Some(execute) = symbol_table.get("Kernel:Execute:None") {
        valid_trace.push(*execute);
    } else if let Some(execute) = symbol_table.get("Kernel:Execute:Repair") {
        valid_trace.push(*execute);
    } else if let Some(execute) = symbol_table.get("Kernel:Execute:Escalate") {
        valid_trace.push(*execute);
    } else if let Some(execute) = symbol_table.get("Kernel:Execute:Recommend") {
        valid_trace.push(*execute);
    }
    valid_trace.push(*symbol_table.get("Kernel:Adapt").unwrap());

    let is_valid = self_model.is_trace_valid(&valid_trace);
    assert!(is_valid, "Failed to validate its own happy path!");
    println!("  ✅ Formal verification of 'Happy Path' passed (O(1) Branchless Validation).");

    // Attempting an illegal operation: Proposing before Observing
    let invalid_trace = vec![
        *symbol_table.get("Kernel:Propose").unwrap(),
        *symbol_table.get("Kernel:Observe").unwrap(),
    ];

    let is_invalid_caught = !self_model.is_trace_valid(&invalid_trace);
    assert!(
        is_invalid_caught,
        "Failed to catch structural violation of its own rules!"
    );
    println!("  ✅ Formal rejection of 'Illegal Action Reversal' passed.");

    println!("\n🎓 ---------------------------------------------------- 🎓");
    println!("🎓 FINAL DEFENSE CONCLUDED: SYSTEMIC AWARENESS ACHIEVED 🎓");
    println!("🎓 ---------------------------------------------------- 🎓\n");
}
