use divan::black_box;
use dteam::dteam::orchestration::Engine;
use dteam::models::{Event, EventLog, Trace};

fn main() {
    divan::main();
}

fn bench_degenerate_loops_inner(n: usize) {
    let mut log = EventLog::default();
    let mut trace = Trace::default();
    for _ in 0..n {
        trace.events.push(Event::new("loop_act".to_string()));
    }
    log.add_trace(trace);
    let engine = Engine::builder().build();
    black_box(engine.run(black_box(&log)));
}

#[divan::bench]
fn bench_degenerate_loops_min() {
    bench_degenerate_loops_inner(1);
}

#[divan::bench]
fn bench_degenerate_loops_standard() {
    bench_degenerate_loops_inner(1000);
}

#[divan::bench]
fn bench_degenerate_loops_max() {
    bench_degenerate_loops_inner(10000);
}

fn bench_disconnected_components_inner(n: usize) {
    let mut log = EventLog::default();
    for i in 0..n {
        let mut trace = Trace::default();
        trace.events.push(Event::new(format!("act_{}", i)));
        log.add_trace(trace);
    }
    let engine = Engine::builder().build();
    black_box(engine.run(black_box(&log)));
}

#[divan::bench]
fn bench_disconnected_components_min() {
    bench_disconnected_components_inner(1);
}

#[divan::bench]
fn bench_disconnected_components_standard() {
    bench_disconnected_components_inner(100);
}

#[divan::bench]
fn bench_disconnected_components_max() {
    bench_disconnected_components_inner(1000);
}
