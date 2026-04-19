use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wasm4pm::models::{EventLog, Trace, Event};
use wasm4pm::dpie::orchestration::{Engine, KTier};

fn create_log_of_size(n: usize) -> EventLog {
    let mut log = EventLog::new();
    let mut trace = Trace::new("t1".into());
    for i in 0..n {
        trace.events.push(Event::new(format!("act_{}", i)));
    }
    log.add_trace(trace);
    log
}

fn bench_ktier_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("K-Tier_Scalability");
    
    // Scale from 50 to 500 activities
    for size in [50, 100, 250, 500].iter() {
        let log = create_log_of_size(*size);
        
        group.bench_with_input(format!("Engine::run/K-Size_{}", size), size, |b, &s| {
            // Pre-pass will select appropriate tier (K64, K128, K256, K512)
            let engine = Engine::builder().with_k_tier(s).build();
            b.iter(|| engine.run(black_box(&log)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_ktier_scalability);
criterion_main!(benches);
