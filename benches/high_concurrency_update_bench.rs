use divan::black_box;
use dteam::dteam::orchestration::Engine;
use dteam::models::{Event, EventLog, Trace};

fn main() {
    divan::main();
}

fn create_batch(num_logs: usize, events_per_log: usize) -> Vec<EventLog> {
    (0..num_logs)
        .map(|_| {
            let mut log = EventLog::default();
            let mut trace = Trace::default();
            for _ in 0..events_per_log {
                trace.events.push(Event::new("act".to_string()));
            }
            log.add_trace(trace);
            log
        })
        .collect()
}

#[divan::bench]
fn bench_batch_concurrency_min() {
    let engine = Engine::builder().build();
    let logs = create_batch(1, 1);
    black_box(engine.run_batch(black_box(&logs)));
}

#[divan::bench]
fn bench_batch_concurrency_standard() {
    let engine = Engine::builder().build();
    let logs = create_batch(10, 1);
    black_box(engine.run_batch(black_box(&logs)));
}

#[divan::bench]
fn bench_batch_concurrency_max() {
    let engine = Engine::builder().build();
    let logs = create_batch(100, 10);
    black_box(engine.run_batch(black_box(&logs)));
}
