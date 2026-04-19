use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wasm4pm::dpie::orchestration::Engine;
use wasm4pm::models::{EventLog, Trace, Event};

fn create_large_log(n: usize) -> EventLog {
    let mut log = EventLog::new();
    let mut trace = Trace::new("t1".into());
    for i in 0..n {
        trace.events.push(Event::new(format!("act_{}", i)));
    }
    log.add_trace(trace);
    log
}

fn bench_e2e_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("E2E_Orchestration");
    
    let log = create_large_log(50); // Small but representative for 1 epoch
    let engine = Engine::builder()
        .with_k_tier(64)
        .build();

    group.bench_function("Engine::run/1_epoch", |b| b.iter(|| {
        engine.run(black_box(&log))
    }));

    group.finish();
}

criterion_group!(benches, bench_e2e_discovery);
criterion_main!(benches);
