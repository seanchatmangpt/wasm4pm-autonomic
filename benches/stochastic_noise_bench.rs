use divan::black_box;
use dteam::models::{EventLog, Trace, Event};
use dteam::dteam::orchestration::Engine;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_resilience_to_noise() {
    let mut log = EventLog::default();
    let mut trace = Trace::default();
    // Insert "noisy" activities that don't match formal process models
    trace.events.push(Event::new("noise_act_1".to_string()));
    trace.events.push(Event::new("valid_act".to_string()));
    trace.events.push(Event::new("noise_act_2".to_string()));
    log.add_trace(trace);

    let engine = Engine::builder().build();
    // Engine should classify correctly despite noise
    black_box(engine.run(black_box(&log)));
}
