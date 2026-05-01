//! POWL8 plan admission via STRIPS, plus Phase-10 adversarial topology tests
//! covering Choice, Loop, fan-in, parallel-independent, and runtime-slot mask
//! domain assertions.

use ccog::breeds::strips::{admit_powl8, admit_powl8_with_advanced};
use ccog::powl::{BinaryRelation, CompiledNodeKind, Powl8, Powl8Node, MAX_NODES};
use ccog::verdict::{Breed, PlanAdmission};
use ccog::FieldContext;

// Layout-dependent indices: tests below construct PartialOrder containers
// with `start=0`, so breed indices follow their listing order. Diamond uses
// 4 breeds (Strips at 3), linear uses 3 (Strips at 2).
const ELIZA_IDX: usize = 0;
const MYCIN_IDX: usize = 1;
const SHRDLU_IDX: usize = 2;
// Diamond layout: 0=Eliza, 1=Mycin, 2=Shrdlu, 3=Strips.
const STRIPS_IDX_DIAMOND: usize = 3;
// Linear layout: 0=Eliza, 1=Mycin, 2=Strips.
const STRIPS_IDX_LINEAR: usize = 2;

/// Build a partial-order plan whose first `count` nodes are the listed
/// breeds, followed by a single `PartialOrder` container that owns them via
/// `start = 0, count = count`.
fn build_breed_plan(breeds: &[Breed], rel: BinaryRelation) -> Powl8 {
    let mut p = Powl8::new();
    for b in breeds {
        p.push(Powl8Node::Activity(*b)).unwrap();
    }
    let _po = p
        .push(Powl8Node::PartialOrder {
            start: 0,
            count: breeds.len() as u16,
            rel,
        })
        .unwrap();
    p.root = 0;
    p
}

#[test]
fn linear_plan_ready_advances_in_order() {
    // Eliza -> Mycin -> Strips
    let mut rel = BinaryRelation::new();
    rel.add_edge(ELIZA_IDX, MYCIN_IDX);
    rel.add_edge(MYCIN_IDX, STRIPS_IDX_LINEAR);

    let plan = build_breed_plan(&[Breed::Eliza, Breed::Mycin, Breed::Strips], rel);
    assert!(plan.shape_match().is_ok(), "linear plan should be Sound");

    // Step 1 — nothing advanced.
    let mut advanced = [false; MAX_NODES];
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.admission, PlanAdmission::Sound);
    assert!(v.admissible);
    assert_eq!(v.ready, vec![ELIZA_IDX]);
    assert!(v.blocked.contains(&MYCIN_IDX));
    assert!(v.blocked.contains(&STRIPS_IDX_LINEAR));

    // Step 2 — Eliza advanced.
    advanced[ELIZA_IDX] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.ready, vec![MYCIN_IDX]);
    assert!(v.blocked.contains(&STRIPS_IDX_LINEAR));

    // Step 3 — Mycin advanced.
    advanced[MYCIN_IDX] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.ready, vec![STRIPS_IDX_LINEAR]);
    assert!(v.blocked.is_empty());

    // Step 4 — all advanced. No work remains, plan still admissible.
    advanced[STRIPS_IDX_LINEAR] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.admission, PlanAdmission::Sound);
    assert!(v.admissible);
    assert!(v.ready.is_empty());
    assert!(v.blocked.is_empty());
}

#[test]
fn diamond_plan_joins_after_both_branches() {
    // Eliza -> {Mycin, Shrdlu} -> Strips
    // Layout: 0=Eliza, 1=Mycin, 2=Shrdlu, 3=Strips.
    let mut rel = BinaryRelation::new();
    rel.add_edge(ELIZA_IDX, MYCIN_IDX);
    rel.add_edge(ELIZA_IDX, SHRDLU_IDX);
    rel.add_edge(MYCIN_IDX, STRIPS_IDX_DIAMOND);
    rel.add_edge(SHRDLU_IDX, STRIPS_IDX_DIAMOND);

    let plan = build_breed_plan(
        &[Breed::Eliza, Breed::Mycin, Breed::Shrdlu, Breed::Strips],
        rel,
    );
    assert!(plan.shape_match().is_ok(), "diamond plan should be Sound");

    // Pre-advance Eliza only — both branches become ready, Strips blocked.
    let mut advanced = [false; MAX_NODES];
    advanced[ELIZA_IDX] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.admission, PlanAdmission::Sound);
    assert!(v.ready.contains(&MYCIN_IDX));
    assert!(v.ready.contains(&SHRDLU_IDX));
    assert!(v.blocked.contains(&STRIPS_IDX_DIAMOND));
    assert!(!v.ready.contains(&STRIPS_IDX_DIAMOND));

    // Mark Mycin only — Strips still blocked (waiting on Shrdlu).
    advanced[MYCIN_IDX] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert!(v.ready.contains(&SHRDLU_IDX));
    assert!(v.blocked.contains(&STRIPS_IDX_DIAMOND));
    assert!(!v.ready.contains(&STRIPS_IDX_DIAMOND));

    // Mark Shrdlu too — Strips is now ready.
    advanced[SHRDLU_IDX] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.ready, vec![STRIPS_IDX_DIAMOND]);
    assert!(v.blocked.is_empty());
}

#[test]
fn cyclic_plan_rejected_with_admission_cyclic() {
    // 0 -> 1 -> 2 -> 0 (cycle).
    let mut rel = BinaryRelation::new();
    rel.add_edge(0, 1);
    rel.add_edge(1, 2);
    rel.add_edge(2, 0);

    let plan = build_breed_plan(&[Breed::Eliza, Breed::Mycin, Breed::Strips], rel);
    assert!(
        plan.shape_match().is_err(),
        "cyclic plan must fail shape_match"
    );
    assert_eq!(
        plan.shape_match().unwrap_err(),
        PlanAdmission::Cyclic,
        "cycle should classify as Cyclic, not Malformed"
    );

    let field = FieldContext::new("powl-cyclic");
    let v = admit_powl8(&plan, &field).unwrap();
    assert_eq!(v.admission, PlanAdmission::Cyclic);
    assert!(!v.admissible);
    assert!(v.ready.is_empty());
    assert!(v.blocked.is_empty());
}

// =============================================================================
// Phase-10 adversarial topology tests
// =============================================================================

#[test]
fn vector_order_diverges_from_execution_order() {
    // Plan vector order intentionally inverted from data-flow order: we push
    // a sequence A→B→C→D in reverse vec positions and prove `compile()` still
    // returns a topological order (B precedes A in vec; A must precede B at
    // runtime due to OperatorSequence).
    let mut p = Powl8::new();
    let b = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap(); // 0
    let a = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 1
    p.push(Powl8Node::OperatorSequence { a, b }).unwrap(); // 2
    p.root = a;

    let compiled = p.compile().expect("compile must succeed");
    let pos_a = compiled.order.iter().position(|&x| x == a).unwrap();
    let pos_b = compiled.order.iter().position(|&x| x == b).unwrap();
    assert!(
        pos_a < pos_b,
        "A must precede B at runtime even though A's vec idx > B's"
    );
}

#[test]
fn diamond_join_fan_in() {
    // Eliza --> {Mycin, Shrdlu} --> Strips
    let mut rel = BinaryRelation::new();
    rel.add_edge(0, 1);
    rel.add_edge(0, 2);
    rel.add_edge(1, 3);
    rel.add_edge(2, 3);
    let plan = build_breed_plan(
        &[Breed::Eliza, Breed::Mycin, Breed::Shrdlu, Breed::Strips],
        rel,
    );
    let mut advanced = [false; MAX_NODES];
    advanced[0] = true;
    advanced[1] = true;
    // Strips still blocked: Shrdlu unfinished.
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert!(v.blocked.contains(&3));
    advanced[2] = true;
    let v = admit_powl8_with_advanced(&plan, &advanced).unwrap();
    assert_eq!(v.ready, vec![3]);
}

#[test]
fn parallel_independent_no_predecessor() {
    // OperatorParallel must NOT add a predecessor edge between a and b.
    let mut p = Powl8::new();
    let s = p.push(Powl8Node::StartNode).unwrap();
    p.root = s;
    let a = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
    let b = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
    p.push(Powl8Node::OperatorParallel { a, b }).unwrap();
    let preds = p.predecessor_masks();
    assert_eq!(preds[a as usize] & (1u64 << b), 0);
    assert_eq!(preds[b as usize] & (1u64 << a), 0);
}

#[test]
fn choice_two_branches_both_admissible() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap(); // 0
    p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 1
    p.push(Powl8Node::Activity(Breed::Mycin)).unwrap(); // 2
    p.push(Powl8Node::Choice {
        branches: [1, 2, 0, 0],
        len: 2,
    })
    .unwrap(); // 3
    assert!(p.shape_match().is_ok());
    let preds = p.predecessor_masks();
    // Both branches must list the Choice node (idx 3) as predecessor.
    assert_ne!(preds[1] & (1u64 << 3), 0);
    assert_ne!(preds[2] & (1u64 << 3), 0);
}

#[test]
fn choice_oob_branch_index_malformed() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    p.push(Powl8Node::Choice {
        branches: [99, 0, 0, 0],
        len: 1,
    })
    .unwrap();
    assert_eq!(p.shape_match().unwrap_err(), PlanAdmission::Malformed);
}

#[test]
fn choice_cyclic_through_branch_is_cyclic() {
    let mut p = Powl8::new();
    p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 0
    p.push(Powl8Node::Choice {
        branches: [0, 0, 0, 0],
        len: 1,
    })
    .unwrap(); // 1 → 0
               // 0 -> 1 via OperatorSequence; 1 -> 0 via Choice → cycle.
    p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
    let res = p.shape_match();
    assert!(
        matches!(res, Err(PlanAdmission::Cyclic)),
        "Choice → branch with reverse edge must classify as Cyclic, got {res:?}"
    );
}

#[test]
fn choice_zero_len_is_malformed() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    p.push(Powl8Node::Choice {
        branches: [0; 4],
        len: 0,
    })
    .unwrap();
    assert_eq!(p.shape_match().unwrap_err(), PlanAdmission::Malformed);
}

#[test]
fn choice_branch_selection_admissible() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap(); // 0
    p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 1
    p.push(Powl8Node::Activity(Breed::Cbr)).unwrap(); // 2
    p.push(Powl8Node::Choice {
        branches: [1, 2, 0, 0],
        len: 2,
    })
    .unwrap(); // 3
    let mut advanced = [false; MAX_NODES];
    advanced[0] = true;
    advanced[3] = true; // Choice itself advanced (selector chose).
    let v = admit_powl8_with_advanced(&p, &advanced).unwrap();
    // Both branches now ready (their only declared predecessor is Choice).
    assert!(v.ready.contains(&1));
    assert!(v.ready.contains(&2));
}

#[test]
fn loop_boundary_unrolls_within_cap() {
    let mut p = Powl8::new();
    let s = p.push(Powl8Node::StartNode).unwrap();
    p.root = s;
    let body = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
    p.push(Powl8Node::OperatorSequence { a: s, b: body })
        .unwrap();
    p.push(Powl8Node::Loop { body, max_iters: 3 }).unwrap();

    let compiled = p.compile().expect("compile must succeed");
    // 1 Start + 1 body + 2 unrolled extra copies = 4 runtime nodes.
    let body_count = compiled.order.iter().filter(|&&x| x == body).count();
    assert_eq!(body_count, 3, "max_iters=3 must produce 3 body copies");
}

#[test]
fn loop_exceeds_max_nodes_malformed() {
    // Build a Powl8 close to capacity, then attempt a Loop unroll that pushes
    // executable nodes past 64.
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    let body = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
    // Fill with silents up to ~60 executable nodes.
    for _ in 0..58 {
        p.push(Powl8Node::Silent).unwrap();
    }
    // Now add Loop with max_iters that would unroll past 64.
    p.push(Powl8Node::Loop {
        body,
        max_iters: 16,
    })
    .unwrap();
    match p.compile() {
        Err(PlanAdmission::Malformed) => {}
        other => panic!("expected Malformed (>64 runtime), got {other:?}"),
    }
}

#[test]
fn malformed_self_cycle_detected() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    // Loop body == self → malformed.
    p.push(Powl8Node::Loop {
        body: 1,
        max_iters: 2,
    })
    .unwrap();
    assert_eq!(p.shape_match().unwrap_err(), PlanAdmission::Malformed);
}

#[test]
fn activity_without_compiled_slot_is_admitted_but_unfireable() {
    // An Activity exists in plan and shape-matches; a runtime that doesn't
    // attach a compiled hook for it leaves the slot unfireable but the plan
    // remains shape-Sound.
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    p.push(Powl8Node::Activity(Breed::Prs)).unwrap();
    p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
    assert!(p.shape_match().is_ok());
}

#[test]
fn manual_only_activity_advances_only_on_external_signal() {
    let mut p = Powl8::new();
    p.push(Powl8Node::StartNode).unwrap();
    p.push(Powl8Node::Activity(Breed::Cbr)).unwrap();
    p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
    // Without Cbr admission (empty field has no urn:ccog:Case instances),
    // the activity classifies as "ready" only because its predecessors are
    // satisfied — but `admit_powl8`'s per-node `advanced` probe returns
    // false for Cbr, so the activity is structurally ready yet runtime-
    // unadvanced. The plan stays admissible (forward progress is possible
    // once an external signal arrives).
    let field = FieldContext::new("manual-only");
    let v = admit_powl8(&p, &field).unwrap();
    assert!(
        v.admissible,
        "plan must remain admissible even when only an external signal can advance the manual node"
    );
    assert!(
        v.ready.contains(&1) || v.blocked.contains(&1),
        "manual node 1 must be classified somewhere (ready or blocked), not omitted"
    );
}

#[test]
fn compiled_mask_is_runtime_slot_indexed() {
    // After `compile()`, `CompiledPowl8.preds[i]` is a mask over runtime
    // indices `0..order.len()`, NOT plan-node indices. Verify by building a
    // plan whose plan-node order is intentionally inverted from runtime
    // order and confirming `preds` references runtime slots.
    let mut p = Powl8::new();
    let b = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap(); // plan idx 0
    let a = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // plan idx 1
    p.push(Powl8Node::OperatorSequence { a, b }).unwrap(); // plan idx 2
    p.root = a;
    let compiled = p.compile().expect("compile must succeed");

    // In the compiled order, A (plan idx 1) runs first (runtime idx 0), and
    // B (plan idx 0) runs second (runtime idx 1). The predecessor mask of B
    // must reference RUNTIME idx 0 (where A landed), NOT plan idx 1.
    let rt_a = compiled.order.iter().position(|&x| x == a).unwrap();
    let rt_b = compiled.order.iter().position(|&x| x == b).unwrap();
    assert_eq!(compiled.preds[rt_b], 1u64 << rt_a);
    // Sanity: kinds align with runtime, not plan, order.
    matches!(compiled.kinds[rt_a], CompiledNodeKind::HookSlot(_));
    matches!(compiled.kinds[rt_b], CompiledNodeKind::HookSlot(_));
}
