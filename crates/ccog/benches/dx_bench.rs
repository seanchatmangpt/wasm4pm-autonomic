use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::hooks::{HookAct, HookCheck, HookTrigger, KnowledgeHook};
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::runtime::ClosedFieldContext;
use ccog::{compile_builtin, BarkKernel, Construct8};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxigraph::model::{NamedNode, Term, Triple};

use std::sync::Arc;

fn empty_context(snap: Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
    ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    }
}

/// Generate a linear graph: (node_i, next, node_{i+1})
fn generate_linear_graph(size: usize) -> FieldContext {
    let mut field = FieldContext::new("linear");
    let mut nt = String::new();
    for i in 0..size {
        nt.push_str(&format!(
            "<http://example.org/node/{}> <http://example.org/next> <http://example.org/node/{}> .\n",
            i, i + 1
        ));
    }
    field.graph.load_ntriples(&nt).unwrap();
    field
}

/// Generate a star graph: (center, edge, leaf_i)
fn generate_star_graph(size: usize) -> FieldContext {
    let mut field = FieldContext::new("star");
    let mut nt = String::new();
    for i in 0..size {
        nt.push_str(&format!(
            "<http://example.org/center> <http://example.org/edge> <http://example.org/leaf/{}> .\n",
            i
        ));
    }
    field.graph.load_ntriples(&nt).unwrap();
    field
}

fn bench_graph_topologies(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_topologies");
    for size in [10, 100, 1000].iter() {
        // Linear
        group.bench_with_input(BenchmarkId::new("linear", size), size, |b, &s| {
            let field = generate_linear_graph(s);
            b.iter(|| {
                let snap = CompiledFieldSnapshot::from_field(black_box(&field)).unwrap();
                black_box(snap)
            });
        });

        // Star
        group.bench_with_input(BenchmarkId::new("star", size), size, |b, &s| {
            let field = generate_star_graph(s);
            b.iter(|| {
                let snap = CompiledFieldSnapshot::from_field(black_box(&field)).unwrap();
                black_box(snap)
            });
        });
    }
    group.finish();
}

fn bench_baseline_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_comparison");

    // Setup ccog environment
    let mut field = FieldContext::new("baseline");
    field.graph.load_ntriples("<http://example.org/subject> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/TargetType> .\n").unwrap();
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let context = empty_context(Arc::new(snap));

    let target_type = NamedNode::new("http://example.org/TargetType").unwrap();

    // Create a simple hook: if TargetType is present, return empty delta.
    let hook = KnowledgeHook {
        name: "baseline_hook",
        trigger: HookTrigger::TypePresent(target_type.clone()),
        check: HookCheck::SnapshotFn(|_ctx| true),
        act: HookAct::ConstantTriples(vec![]),
        emit_receipt: false,
    };
    let kernel = BarkKernel::linear(vec![compile_builtin(&hook).unwrap()]).unwrap();

    group.bench_function("ccog_bark_kernel_fire", |b| {
        b.iter(|| kernel.fire(black_box(&context)).unwrap())
    });

    // Raw Rust comparison
    // We'll simulate what TypePresent does: check if a type ID exists in a set.
    let target_type_id = 12345u64;
    let present_types = vec![target_type_id];

    group.bench_function("raw_rust_match", |b| {
        b.iter(|| {
            let found = match black_box(target_type_id) {
                id if present_types.contains(&id) => true,
                _ => false,
            };
            black_box(found)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_graph_topologies, bench_baseline_comparison);
criterion_main!(benches);
