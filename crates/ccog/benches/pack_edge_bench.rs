//! Edge pack micro-bench (Phase 12). Tier: FullProcess.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBit, PostureBundle};
use ccog::packs::edge::select_instinct;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_edge(c: &mut Criterion) {
    let f = FieldContext::new("bench");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::ALERT,
        confidence: 200,
    };
    let ctx = ContextBundle::default();
    c.bench_function("pack_edge_select_instinct", |b| {
        b.iter(|| {
            let v = select_instinct(black_box(&snap), black_box(&posture), black_box(&ctx));
            black_box(v)
        })
    });
}

criterion_group!(benches, bench_edge);
criterion_main!(benches);
