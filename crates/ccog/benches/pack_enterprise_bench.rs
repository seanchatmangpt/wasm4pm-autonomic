//! Enterprise pack micro-bench (Phase 12). Tier: FullProcess (PROV-rich act fns).

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::packs::enterprise::BUILTINS;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_enterprise(c: &mut Criterion) {
    let f = FieldContext::new("bench");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
    c.bench_function("pack_enterprise_act_first_slot", |b| {
        b.iter(|| {
            let delta = (BUILTINS[0].act)(black_box(&snap)).expect("act");
            black_box(delta)
        })
    });
}

criterion_group!(benches, bench_enterprise);
criterion_main!(benches);
