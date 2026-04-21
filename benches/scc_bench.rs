use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dteam::utils::dense_kernel::KBitSet;
use dteam::utils::scc::{compute_sccs_branchless, compute_sccs_generic};
use fastrand::Rng;

fn generate_random_graph<const WORDS: usize>(density: f64) -> Vec<KBitSet<WORDS>> {
    let mut rng = Rng::with_seed(42);
    let max_nodes = WORDS * 64;
    let mut adj = vec![KBitSet::<WORDS>::zero(); max_nodes];
    for row in adj.iter_mut().take(max_nodes) {
        for j in 0..max_nodes {
            if rng.f64() < density {
                let _ = row.set(j);
            }
        }
    }
    adj
}

fn bench_scc_k64(c: &mut Criterion) {
    let mut group = c.benchmark_group("SCC Detection K64 (64 nodes)");
    let density = 0.3;
    let adj = generate_random_graph::<1>(density);

    group.bench_function("Baseline (Generic)", |b| {
        b.iter(|| compute_sccs_generic(black_box(&adj)))
    });

    group.bench_function("Branchless", |b| {
        b.iter(|| compute_sccs_branchless(black_box(&adj)))
    });

    group.finish();
}

fn bench_scc_k256(c: &mut Criterion) {
    let mut group = c.benchmark_group("SCC Detection K256 (256 nodes)");
    let density = 0.3;
    let adj = generate_random_graph::<4>(density);

    group.bench_function("Baseline (Generic)", |b| {
        b.iter(|| compute_sccs_generic(black_box(&adj)))
    });

    group.bench_function("Branchless", |b| {
        b.iter(|| compute_sccs_branchless(black_box(&adj)))
    });

    group.finish();
}

fn bench_scc_k512(c: &mut Criterion) {
    let mut group = c.benchmark_group("SCC Detection K512 (512 nodes)");
    let density = 0.3;
    let adj = generate_random_graph::<8>(density);

    group.bench_function("Baseline (Generic)", |b| {
        b.iter(|| compute_sccs_generic(black_box(&adj)))
    });

    group.bench_function("Branchless", |b| {
        b.iter(|| compute_sccs_branchless(black_box(&adj)))
    });

    group.finish();
}

fn bench_scc_k1024(c: &mut Criterion) {
    let mut group = c.benchmark_group("SCC Detection K1024 (1024 nodes)");
    let density = 0.3;
    let adj = generate_random_graph::<16>(density);

    group.bench_function("Baseline (Generic)", |b| {
        b.iter(|| compute_sccs_generic(black_box(&adj)))
    });

    group.bench_function("Branchless", |b| {
        b.iter(|| compute_sccs_branchless(black_box(&adj)))
    });

    group.finish();
}

fn bench_scc_density_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("SCC Branchless Density Impact (K64)");

    for density in [0.1, 0.5, 0.9] {
        let adj = generate_random_graph::<1>(density);
        group.bench_with_input(format!("density_{}", density), &adj, |b, adj| {
            b.iter(|| compute_sccs_branchless(black_box(adj)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_scc_k64,
    bench_scc_k256,
    bench_scc_k512,
    bench_scc_k1024,
    bench_scc_density_impact
);
criterion_main!(benches);
