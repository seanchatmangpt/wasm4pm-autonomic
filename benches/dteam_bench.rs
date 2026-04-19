use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dteam::models::{EventLog, Trace, Event};
use dteam::dteam::orchestration::Engine;

fn create_large_log(n: usize) -> EventLog {
    let mut log = EventLog::new();
    let mut trace = Trace::new("t1".into());
    for i in 0..n {
        trace.events.push(Event::new(format!("act_{}", i)));
    }
    log.add_trace(trace);
    log
}

fn bench_dteam_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("DTEAM");
    
    let log = create_large_log(100);

    // 1. Pre-pass sizing
    group.bench_function("PrePass/activity_footprint", |b| b.iter(|| {
        black_box(&log).activity_footprint()
    }));

    // 2. Engine Run (Initialization & Pre-pass)
    let engine = Engine::builder()
        .with_k_tier(128)
        .build();
    group.bench_function("Engine/run_precheck", |b| b.iter(|| {
        engine.run(black_box(&log))
    }));

    group.finish();
}

criterion_group!(benches, bench_dteam_ops);
criterion_main!(benches);
