use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use wasm4pm::ref_conformance::ref_token_replay::apply_token_based_replay_bcinr;
use wasm4pm::ref_models::ref_petri_net::{PetriNet, ArcType};
use wasm4pm::ref_models::ref_event_log::EventLogActivityProjection;

fn setup_model() -> PetriNet {
    let mut net = PetriNet::new();
    net.add_place("p1".into());
    net.add_transition("t1".into(), Some("A".into()));
    net.add_arc_pt("p1".into(), "t1".into());
    net.initial_marking = Some(vec![("p1".into(), 1)].into());
    net.final_markings = Some(vec![vec![("p1".into(), 0)].into()]);
    net
}

#[library_benchmark]
fn bench_replayer_sound() -> u64 {
    let net = setup_model();
    let projection = EventLogActivityProjection {
        activities: vec!["A".into()],
        traces: vec![(vec![0], 1)],
    };
    apply_token_based_replay_bcinr(&net, &projection).consumed
}

#[library_benchmark]
fn bench_replayer_noisy() -> u64 {
    let net = setup_model();
    // B is not in model, should trigger dummy masking
    let projection = EventLogActivityProjection {
        activities: vec!["B".into()],
        traces: vec![(vec![0], 1)],
    };
    apply_token_based_replay_bcinr(&net, &projection).consumed
}

library_benchmark_group!(
    name = stability_group;
    benchmarks = bench_replayer_sound, bench_replayer_noisy
);

main!(library_benchmark_groups = stability_group);
