use divan::black_box;
use dteam::models::{EventLog, Trace, Event};
use dteam::dteam::orchestration::Engine;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_batch_concurrency() {
    let engine = Engine::builder().build();
    let logs: Vec<EventLog> = (0..10).map(|_| {
        let mut log = EventLog::default();
        let mut trace = Trace::default();
        trace.events.push(Event::new("act".to_string()));
        log.add_trace(trace);
        log
    }).collect();

    // Verify batching throughput
    black_box(engine.run_batch(black_box(&logs)));
}
