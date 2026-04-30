//! Lifestyle pack micro-bench (Phase 12).
//!
//! Tier: FullProcess (per CLAUDE.md tier table). The bias wrapper invokes
//! `select_instinct_v0` once and applies an O(1) clamp. We bench the wrapper
//! to make sure pack overhead never grows beyond a single comparison.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::lifestyle::{select_instinct, LifestyleBit};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_lifestyle(c: &mut Criterion) {
    let f = FieldContext::new("bench");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
    let posture = PostureBundle {
        posture_mask: (1u64 << PostureBit::ALERT) | (1u64 << LifestyleBit::FATIGUED),
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 0,
        risk_mask: 1u64 << ContextBit::THEFT_RISK,
        affordance_mask: 0,
    };
    c.bench_function("pack_lifestyle_select_instinct", |b| {
        b.iter(|| {
            let v = select_instinct(black_box(&snap), black_box(&posture), black_box(&ctx));
            black_box(v)
        })
    });
}

criterion_group!(benches, bench_lifestyle);
criterion_main!(benches);
