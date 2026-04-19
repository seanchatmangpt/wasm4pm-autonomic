use wasm4pm::reinforcement::{Agent, SARSAAgent};
use wasm4pm::ref_conformance::ref_token_replay::apply_token_based_replay_bcinr;
use wasm4pm::ref_models::ref_petri_net::{PetriNet, ArcType};
use wasm4pm::ref_models::ref_event_log::EventLogActivityProjection;
use wasm4pm::{RlState, RlAction};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("Starting Zero-Allocation Benchmark...");

    // 1. RL Hot Path
    let agent = SARSAAgent::<RlState, RlAction>::new();
    let state = RlState {
        health_level: 1,
        event_rate_q: 0,
        activity_count_q: 0,
        spc_alert_level: 0,
        drift_status: 0,
        rework_ratio_q: 0,
        circuit_state: 0,
        cycle_phase: 0,
        marking_mask: 1,
        activities_hash: 1,
    };
    
    println!("Executing 1,000,000 RL updates...");
    for _ in 0..1_000_000 {
        let action = agent.select_action(state);
        agent.update(state, action, 1.0, state, false);
    }

    // 2. BCINR Replay Hot Path
    let mut net = PetriNet::new();
    let p1 = net.add_place(None);
    let t1 = net.add_transition(Some("A".into()), None);
    net.add_arc(ArcType::PlaceTransition(p1.0, t1.0), Some(1));
    
    let mut init_marking = HashMap::new();
    init_marking.insert(p1, 1);
    net.initial_marking = Some(init_marking);
    
    let mut final_marking = HashMap::new();
    final_marking.insert(p1, 0);
    net.final_markings = Some(vec![final_marking]);

    let mut act_to_index = HashMap::new();
    act_to_index.insert("A".into(), 0);
    let projection = EventLogActivityProjection {
        activities: vec!["A".into()],
        act_to_index,
        traces: vec![(vec![0], 1000)],
    };

    println!("Executing 1,000 BCINR Replays...");
    for _ in 0..1000 {
        let _ = apply_token_based_replay_bcinr(&net, &projection);
    }

    println!("Benchmark Complete. DHAT will now report allocations.");
}
