use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dteam::agentic::ralph::patterns::universe64::{UCoord, UReceipt, Universe64};

fn bench_universe64_fire(c: &mut Criterion) {
    let mut group = c.benchmark_group("Universe64");

    group.bench_function("fire_t1_admissibility", |b| {
        let mut universe = Universe64::empty();
        
        // Setup initial facts
        universe.data[0] = 0b1010;
        universe.data[5] = 0xFF;

        let word_idx = 0;
        let input_mask = 0b0010; // requires bit 1
        let output_mask = 0b0100; // produces bit 2

        b.iter(|| {
            let mask = universe.apply_local_transition(black_box(word_idx), black_box(input_mask), black_box(output_mask));
            black_box(mask);
        });
    });

    group.bench_function("fire_t1_complex", |b| {
        let mut universe = Universe64::empty();
        
        let mut transitions = Vec::new();
        for i in 0..8 {
            universe.data[i] = 1 << i;
            let input_mask = 1 << i;
            let output_mask = 1 << (i + 1);
            transitions.push((i, input_mask, output_mask));
        }
        
        let mut receipt = UReceipt::new();

        b.iter(|| {
            let count = universe.apply_sparse_transitions(black_box(&transitions), black_box(&mut receipt));
            black_box(count);
        });
    });

    group.bench_function("conformance_distance", |b| {
        let u1 = Universe64::empty();
        let mut u2 = Universe64::empty();
        u2.data[0] = !0;
        u2.data[4095] = !0;

        b.iter(|| {
            let dist = black_box(&u1).conformance_distance(black_box(&u2));
            black_box(dist);
        });
    });

    group.bench_function("apply_boundary_transition", |b| {
        let mut universe = Universe64::empty();
        universe.data[0] = 0b1010;
        universe.data[5] = 0b0000;

        let idx_a = 0;
        let in_a = 0b0010;
        let out_a = 0b0000;

        let idx_b = 5;
        let in_b = 0b0000;
        let out_b = 0b0100;

        b.iter(|| {
            let mask = universe.apply_boundary_transition(
                black_box(idx_a), black_box(in_a), black_box(out_a),
                black_box(idx_b), black_box(in_b), black_box(out_b)
            );
            black_box(mask);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_universe64_fire);
criterion_main!(benches);
