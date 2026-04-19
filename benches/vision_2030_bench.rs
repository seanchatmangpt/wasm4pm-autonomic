use divan::{black_box, Bencher};
use dteam::simd::SwarMarking;
use dteam::probabilistic::CountMinSketch;
use dteam::ml::LinUcb;
use dteam::agentic::Simulator;
use dteam::autonomic::AutonomicState;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_swar_fire(bencher: Bencher) {
    let marking = SwarMarking(0x0000_0000_0000_000F);
    let req = 0x0000_0000_0000_0003;
    let out = 0x0000_0000_0000_0030;
    
    bencher.bench(|| {
        black_box(marking).try_fire_branchless(black_box(req), black_box(out))
    });
}

#[divan::bench]
fn bench_count_min_add(bencher: Bencher) {
    bencher
        .with_inputs(|| CountMinSketch::new(1024, 4))
        .bench_local_refs(|cms| {
            cms.add(black_box("activity_alpha"));
        });
}

#[divan::bench]
fn bench_linucb_select(bencher: Bencher) {
    let bandit: LinUcb<16, 256> = LinUcb::new(0.1);
    let context = [0.5; 16];
    
    bencher.bench(|| {
        bandit.select_action(black_box(&context), black_box(5))
    });
}

#[divan::bench]
fn bench_counterfactual_sim(bencher: Bencher) {
    let state = AutonomicState {
        process_health: 0.9,
        throughput: 100.0,
        conformance_score: 0.95,
        drift_detected: false,
        active_cases: 10,
    };
    let sim = Simulator::new(state);
    let action = dteam::autonomic::AutonomicAction::recommend(1, "test");
    
    bencher.bench(|| {
        sim.evaluate_action(black_box(&action))
    });
}
