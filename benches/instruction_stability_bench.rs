use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use dteam::ref_conformance::ref_token_replay::apply_token_based_replay_bcinr;
use dteam::ref_models::ref_petri_net::{PetriNet, ArcType};
use dteam::ref_models::ref_event_log::EventLogActivityProjection;
use std::collections::HashMap;

fn setup_model() -> PetriNet {
    let mut net = PetriNet::new();
    let p1 = net.add_place(None);
    let t1 = net.add_transition(Some("A".into()), None);
    net.add_arc(ArcType::PlaceTransition(p1.0, t1.0), None);
    
    let mut initial_marking = HashMap::new();
    initial_marking.insert(p1, 1);
    net.initial_marking = Some(initial_marking);
    
    let mut final_marking = HashMap::new();
    final_marking.insert(p1, 0);
    net.final_markings = Some(vec![final_marking]);
    net
}

#[library_benchmark]
fn bench_replayer_sound() -> u64 {
    let net = setup_model();
    let mut act_to_index = HashMap::new();
    act_to_index.insert("A".into(), 0);
    let projection = EventLogActivityProjection {
        activities: vec!["A".into()],
        act_to_index,
        traces: vec![(vec![0], 1)],
    };
    apply_token_based_replay_bcinr(&net, &projection).consumed
}

#[library_benchmark]
fn bench_replayer_noisy() -> u64 {
    let net = setup_model();
    let mut act_to_index = HashMap::new();
    act_to_index.insert("B".into(), 0);
    // B is not in model, should trigger dummy masking
    let projection = EventLogActivityProjection {
        activities: vec!["B".into()],
        act_to_index,
        traces: vec![(vec![0], 1)],
    };
    apply_token_based_replay_bcinr(&net, &projection).consumed
}

library_benchmark_group!(
    name = stability_group;
    benchmarks = bench_replayer_sound, bench_replayer_noisy
);

main!(library_benchmark_groups = stability_group);
