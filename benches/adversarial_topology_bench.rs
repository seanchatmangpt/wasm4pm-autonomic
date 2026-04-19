use divan::black_box;
use dteam::models::{EventLog, Trace, Event};
use dteam::dteam::orchestration::Engine;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_degenerate_loops() {
    let mut log = EventLog::default();
    let mut trace = Trace::default();
    // Simulate infinite loop of a single activity
    for _ in 0..1000 {
        trace.events.push(Event::new("loop_act".to_string()));
    }
    log.add_trace(trace);
    let engine = Engine::builder().build();
    black_box(engine.run(black_box(&log)));
}

#[divan::bench]
fn bench_disconnected_components() {
    let mut log = EventLog::default();
    // Many independent traces
    for i in 0..100 {
        let mut trace = Trace::default();
        trace.events.push(Event::new(format!("act_{}", i)));
        log.add_trace(trace);
    }
    let engine = Engine::builder().build();
    black_box(engine.run(black_box(&log)));
}
