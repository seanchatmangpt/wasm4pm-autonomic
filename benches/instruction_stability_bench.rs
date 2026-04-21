use dteam::ref_conformance::ref_token_replay::apply_token_based_replay_bcinr;
use dteam::ref_models::ref_event_log::EventLogActivityProjection;
use dteam::ref_models::ref_petri_net::{ArcType, PetriNet};
use dteam::utils::dense_kernel::KBitSet;
use dteam::utils::scc::compute_sccs_branchless;
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::collections::HashMap;

fn generate_random_graph<const WORDS: usize>(density: f64) -> Vec<KBitSet<WORDS>> {
<<<<<<< HEAD
    let mut adj = vec![KBitSet::<WORDS>::zero(); WORDS * 64];
    for (i, row) in adj.iter_mut().enumerate().take(WORDS * 64) {
        for j in 0..WORDS * 64 {
=======
    let nodes = WORDS * 64;
    let mut adj = vec![KBitSet::<WORDS>::zero(); nodes];
    for (i, row) in adj.iter_mut().enumerate() {
        for j in 0..nodes {
>>>>>>> wreckit/mdl-refinement-upgrade-structural-scoring-in-src-models-petri-net-rs-to-follow-φ-n-exactly
            // Using a simple deterministic "random" for density
            if ((i * 31 + j * 7) % 100) < (density * 100.0) as usize {
                row.set(j).unwrap();
            }
        }
    }
    adj
}

#[library_benchmark]
fn bench_scc_branchless_sparse() -> Vec<KBitSet<1>> {
    let adj = generate_random_graph::<1>(0.1);
    compute_sccs_branchless(&adj)
}

#[library_benchmark]
fn bench_scc_branchless_dense() -> Vec<KBitSet<1>> {
    let adj = generate_random_graph::<1>(0.9);
    compute_sccs_branchless(&adj)
}

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
    benchmarks = bench_replayer_sound, bench_replayer_noisy, bench_scc_branchless_sparse, bench_scc_branchless_dense
);

main!(library_benchmark_groups = stability_group);
