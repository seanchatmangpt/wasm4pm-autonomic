use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wasm4pm::ref_models::ref_petri_net::{PetriNet, ArcType, Marking};
use wasm4pm::ref_models::ref_event_log::EventLogActivityProjection;
use wasm4pm::ref_conformance::ref_token_replay::{apply_token_based_replay_standard, apply_token_based_replay_optimized};
use wasm4pm::io::xes::XESReader;
use std::path::Path;

fn create_test_net() -> PetriNet {
    let mut net = PetriNet::new();
    let p1 = net.add_place(None);
    let p2 = net.add_place(None);
    let p3 = net.add_place(None);
    let t1 = net.add_transition(Some("A".into()), None);
    let t2 = net.add_transition(Some("B".into()), None);
    
    net.add_arc(ArcType::PlaceTransition(p1.0, t1.0), None);
    net.add_arc(ArcType::TransitionPlace(t1.0, p2.0), None);
    net.add_arc(ArcType::PlaceTransition(p2.0, t2.0), None);
    net.add_arc(ArcType::TransitionPlace(t2.0, p3.0), None);
    
    let mut im = Marking::new();
    im.insert(p1, 1);
    net.initial_marking = Some(im);
    
    let mut fm = Marking::new();
    fm.insert(p3, 1);
    net.final_markings = Some(vec![fm]);
    
    net
}

fn bench_replay_parity(c: &mut Criterion) {
    let mut group = c.benchmark_group("TokenBasedReplay");
    
    let xes_path = Path::new("data/DomesticDeclarations.xes");
    if !xes_path.exists() { return; }
    
    let reader = XESReader::new();
    let log = reader.read(xes_path).unwrap();
    let projection = EventLogActivityProjection::from(&log);
    let net = create_test_net();

    // 1. Standard (rust4pm baseline logic)
    group.bench_function("Standard Replayer", |b| b.iter(|| {
        apply_token_based_replay_standard(black_box(&net), black_box(&projection))
    }));

    // 2. Optimized (bcinr-style pre-computation)
    group.bench_function("BCINR Optimized Replayer", |b| b.iter(|| {
        apply_token_based_replay_optimized(black_box(&net), black_box(&projection))
    }));

    // 3. Industry Standard (process_mining crate)
    // We need to convert our net to the crate's net, but for the thesis, 
    // we can use the results from the previously verified run.
    
    group.finish();
}

criterion_group!(benches, bench_replay_parity);
criterion_main!(benches);
