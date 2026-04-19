use divan::black_box;
use dteam::dteam::orchestration::Engine;
use dteam::models::EventLog;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_pure_rust_engine_run_batch() {
    let engine = Engine::builder().build();
    let logs = vec![EventLog::default(); 100]; 
    black_box(engine.run_batch(black_box(&logs)));
}

#[divan::bench]
fn bench_pure_rust_engine_run() {
    let engine = Engine::builder().build();
    let log = EventLog::default(); // Minimal footprint
    black_box(engine.run(black_box(&log)));
}

#[divan::bench]
fn bench_json_serialization_cost() {
    let log = EventLog::default();
    let json_str = serde_json::to_string(&log).unwrap();
    black_box(serde_json::from_str::<EventLog>(black_box(&json_str)).unwrap());
}

#[divan::bench]
fn bench_boundary_crossing_overhead() {
    // Simulate crossing the WASM boundary (n calls)
    for _ in 0..1000 {
        black_box(());
    }
}
