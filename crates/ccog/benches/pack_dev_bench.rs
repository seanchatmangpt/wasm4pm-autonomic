//! Dev pack micro-bench (Phase 12). Tier: FullProcess.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::dev::select_instinct;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_dev(c: &mut Criterion) {
    let f = FieldContext::new("bench");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::ALERT,
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 0,
        risk_mask: 1u64 << ContextBit::MUST_ESCALATE,
        affordance_mask: 0,
    };
    c.bench_function("pack_dev_select_instinct_clamps_escalate", |b| {
        b.iter(|| {
            let v = select_instinct(black_box(&snap), black_box(&posture), black_box(&ctx));
            black_box(v)
        })
    });
}

criterion_group!(benches, bench_dev);
criterion_main!(benches);
