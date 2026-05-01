//! Fuzzing the COG8 bounded cognitive closure kernel (PRD v0.4).
//!
//! This test uses `proptest` to fuzz `execute_cog8_graph` with random field
//! snapshots and random graph topologies. It searches for:
//! 1. Panic conditions (out-of-bounds, overflows).
//! 2. Memory leaks (though the kernel is allocation-free).
//! 3. Correctness of attribution (highest priority wins).
//! 4. Terminal state lawfulness.
//! 5. Convergence and budget violations.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::runtime::cog8::*;
use ccog::runtime::ClosedFieldContext;
use proptest::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

// =============================================================================
// Allocation-counting GlobalAlloc
// =============================================================================

struct CountingAlloc;

thread_local! {
    static TL_OCTETS: Cell<u64> = const { Cell::new(0) };
    static TL_COUNT: Cell<u64> = const { Cell::new(0) };
    static TL_ENABLED: Cell<bool> = const { Cell::new(false) };
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = TL_ENABLED.try_with(|e| {
            if e.get() {
                TL_OCTETS.with(|b| b.set(b.get() + layout.size() as u64));
                TL_COUNT.with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn measure_alloc<R>(f: impl FnOnce() -> R) -> (R, u64, u64) {
    TL_OCTETS.with(|b| b.set(0));
    TL_COUNT.with(|c| c.set(0));
    TL_ENABLED.with(|e| e.set(true));
    let r = f();
    TL_ENABLED.with(|e| e.set(false));
    let octets = TL_OCTETS.with(|b| b.get());
    let count = TL_COUNT.with(|c| c.get());
    (r, octets, count)
}

// =============================================================================
// Strategies
// =============================================================================

fn arb_instinct() -> impl Strategy<Value = Instinct> {
    prop_oneof![
        Just(Instinct::Settle),
        Just(Instinct::Retrieve),
        Just(Instinct::Inspect),
        Just(Instinct::Ask),
        Just(Instinct::Refuse),
        Just(Instinct::Escalate),
        Just(Instinct::Ignore),
    ]
}

fn arb_collapse_fn() -> impl Strategy<Value = CollapseFn> {
    prop_oneof![
        Just(CollapseFn::None),
        Just(CollapseFn::ReflectivePosture),
        Just(CollapseFn::ExpertRule),
        Just(CollapseFn::Preconditions),
        Just(CollapseFn::Grounding),
        Just(CollapseFn::RelationalProof),
        Just(CollapseFn::Reconstruction),
        Just(CollapseFn::BlackboardFusion),
        Just(CollapseFn::DifferenceReduction),
        Just(CollapseFn::Chunking),
        Just(CollapseFn::ReactiveIntention),
        Just(CollapseFn::CaseAnalogy),
    ]
}

fn arb_powl8_op() -> impl Strategy<Value = Powl8Op> {
    prop_oneof![
        Just(Powl8Op::Act),
        Just(Powl8Op::Choice),
        Just(Powl8Op::Partial),
        Just(Powl8Op::Join),
        Just(Powl8Op::Loop),
        Just(Powl8Op::Silent),
        Just(Powl8Op::Block),
        Just(Powl8Op::Emit),
    ]
}

fn arb_edge_kind() -> impl Strategy<Value = EdgeKind> {
    prop_oneof![
        Just(EdgeKind::Choice),
        Just(EdgeKind::PartialOrder),
        Just(EdgeKind::Loop),
        Just(EdgeKind::Silent),
        Just(EdgeKind::Override),
        Just(EdgeKind::Blocking),
        Just(EdgeKind::None),
    ]
}

fn arb_cog8_row() -> impl Strategy<Value = Cog8Row> {
    (
        any::<u16>(), // pack_id
        any::<u16>(), // group_id
        any::<u16>(), // rule_id
        any::<u8>(),  // breed_id
        arb_collapse_fn(),
        prop::array::uniform8(any::<u16>()), // var_ids
        any::<u64>(),                        // required_mask
        any::<u64>(),                        // forbidden_mask
        any::<u64>(),                        // predecessor_mask
        arb_instinct(),
        any::<u16>(), // priority
    )
        .prop_map(|(p, g, r, b, c, v, req, forb, pred, inst, prio)| {
            let mut var_ids = [FieldId(0); 8];
            for i in 0..8 {
                var_ids[i] = FieldId(v[i]);
            }
            Cog8Row {
                pack_id: PackId(p),
                group_id: GroupId(g),
                rule_id: RuleId(r),
                breed_id: BreedId(b),
                collapse_fn: c,
                var_ids,
                required_mask: req,
                forbidden_mask: forb,
                predecessor_mask: pred,
                response: inst,
                priority: prio,
            }
        })
}

fn arb_powl8_instr(num_nodes: u16, num_edges: u16) -> impl Strategy<Value = Powl8Instr> {
    (
        arb_powl8_op(),
        arb_collapse_fn(),
        (0..num_nodes).prop_map(NodeId),
        (0..num_edges).prop_map(EdgeId),
        any::<u64>(), // guard_mask
        any::<u64>(), // effect_mask
    )
        .prop_map(|(op, col, node, edge, guard, effect)| Powl8Instr {
            op,
            collapse_fn: col,
            node_id: node,
            edge_id: edge,
            guard_mask: guard,
            effect_mask: effect,
        })
}

fn arb_cog8_edge(num_nodes: u16, num_edges: u16) -> impl Strategy<Value = Cog8Edge> {
    (
        (0..num_nodes).prop_map(NodeId),
        (0..num_nodes).prop_map(NodeId),
        arb_edge_kind(),
        arb_powl8_instr(num_nodes, num_edges),
    )
        .prop_map(|(f, t, k, i)| Cog8Edge {
            from: f,
            to: t,
            kind: k,
            instr: i,
        })
}

fn arb_graph() -> impl Strategy<Value = (Vec<Cog8Row>, Vec<Cog8Edge>, u64, u64)> {
    (1..64usize).prop_flat_map(|num_nodes| {
        (
            prop::collection::vec(arb_cog8_row(), num_nodes..num_nodes + 1),
            prop::collection::vec(arb_cog8_edge(num_nodes as u16, 128), 0..128),
            any::<u64>(), // present
            any::<u64>(), // completed_init
        )
    })
}

// =============================================================================
// Fuzz Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    #[test]
    fn prop_kernel_fuzz(
        (nodes, edges, present, completed_init) in arb_graph()
    ) {
        // Warmup: ensure OnceLock initializations are performed.
        let _ = execute_cog8_graph(&nodes, &edges, present, completed_init);

        // Single-pass execution as used in BarkKernel.
        let (decision, octets, _) = measure_alloc(|| {
            execute_cog8_graph(&nodes, &edges, present, completed_init).unwrap()
        });

        // 1. Allocation budget: must be ZERO
        prop_assert_eq!(octets, 0, "execute_cog8_graph must allocate ZERO octets; saw {}", octets);

        // 2. Terminal state lawfulness: completed_mask must contain completed_init
        prop_assert!((decision.completed_mask & completed_init) == completed_init,
            "completed_mask must be monotonic: init={:b}, final={:b}", completed_init, decision.completed_mask);

        // 3. Attribution correctness: if a response is selected, it must be the highest priority among fired nodes.
        if let Some(node_id) = decision.selected_node {
            let idx = node_id.0 as usize;
            prop_assert!(idx < nodes.len());
            prop_assert!((decision.fired_mask & (1u64 << idx)) != 0);
            prop_assert_eq!(decision.response, nodes[idx].response);

            let selected_priority = nodes[idx].priority;
            for (i, node) in nodes.iter().enumerate() {
                if (decision.fired_mask & (1u64 << i)) != 0 {
                    prop_assert!(node.priority <= selected_priority,
                        "Node {} has higher priority {} than selected node {} priority {}",
                        i, node.priority, idx, selected_priority);
                }
            }
        } else {
            prop_assert_eq!(decision.response, Instinct::Ignore);
            prop_assert_eq!(decision.fired_mask, 0, "If no node selected, fired_mask should ideally be 0 (in this single-pass context)");
        }

        // 4. Convergence check (multi-pass)
        let mut completed = completed_init;
        let mut budget = 100;
        while budget > 0 {
            let d = execute_cog8_graph(&nodes, &edges, present, completed).unwrap();
            if d.completed_mask == completed {
                break;
            }
            completed = d.completed_mask;
            budget -= 1;
        }
        prop_assert!(budget > 0, "Budget violation: graph failed to converge in 100 iterations");
    }
}
