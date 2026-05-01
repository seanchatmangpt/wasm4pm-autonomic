use ccog::runtime::cog8::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Zero-allocation verification hook.
struct TrackingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: TrackingAllocator = TrackingAllocator;

fn kernel_bench(c: &mut Criterion) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // 64 nodes to fill the u64 mask - representative of a moderately complex graph
    for i in 0..64 {
        nodes.push(Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(i as u16),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [
                FieldId(0),
                FieldId(1),
                FieldId(2),
                FieldId(3),
                FieldId(4),
                FieldId(5),
                FieldId(6),
                FieldId(7),
            ],
            required_mask: 1 << (i % 8),
            forbidden_mask: 0,
            predecessor_mask: if i > 0 { 1 << (i - 1) } else { 0 },
            response: Instinct::Settle,
            priority: i as u16,
        });

        edges.push(Cog8Edge {
            from: NodeId(if i > 0 { (i - 1) as u16 } else { 0 }),
            to: NodeId(i as u16),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Act,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(i as u16),
                edge_id: EdgeId(i as u16),
                guard_mask: if i > 0 { 1 << (i - 1) } else { 0 },
                effect_mask: 1 << i,
            },
        });
    }

    let present = 0b11111111; // All required bits for any node are present
    let completed = 0;

    c.bench_function("execute_cog8_graph_64_nodes", |b| {
        b.iter(|| {
            let start_count = ALLOC_COUNT.load(Ordering::Relaxed);

            let res = execute_cog8_graph(
                black_box(&nodes),
                black_box(&edges),
                black_box(present),
                black_box(completed),
            );

            let end_count = ALLOC_COUNT.load(Ordering::Relaxed);
            if start_count != end_count {
                panic!(
                    "Heap allocation detected in execute_cog8_graph hot path! ({} -> {})",
                    start_count, end_count
                );
            }

            black_box(res)
        })
    });
}

fn l3_arena_10m_bench(c: &mut Criterion) {
    let arena = ccog::runtime::tournament::l3_arena::L3ProcessCityArena::new();
    let present_mask = 0b11111111_11111111;

    c.bench_function("l3_arena_1k_batch", |b| {
        b.iter(|| {
            let start_count = ALLOC_COUNT.load(Ordering::Relaxed);

            // Execute a 1k batch per iteration. Criterion will scale the number of iterations
            // automatically to achieve statistical significance within the 3-second window.
            for i in 0..1000 {
                let completed = (i % 1024) as u64;
                let res = execute_cog8_graph(
                    black_box(&arena.nodes[..]),
                    black_box(&arena.edges[..]),
                    black_box(present_mask),
                    black_box(completed),
                );
                let _ = black_box(res);
            }

            let end_count = ALLOC_COUNT.load(Ordering::Relaxed);
            if start_count != end_count {
                panic!("Heap allocation detected in L3 1k batch!");
            }
        })
    });
}

criterion_group!(benches, kernel_bench, l3_arena_10m_bench);
criterion_main!(benches);
