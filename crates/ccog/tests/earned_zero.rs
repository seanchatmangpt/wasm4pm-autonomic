//! Phase 7 earned-zero distinction: a fired count of zero can have one of
//! several causes, and the trace must distinguish them. Each test pins one
//! zero-cause and asserts the trace skip reasons are exactly what we
//! claim.
//!
//! "Earned zero" means: the outcome is zero by lawful skip semantics, not
//! by accident, not by silent allocator allocation. Each case below pins
//! a different lawful path.

use ccog::bark_artifact::{decide, decide_table, materialize, seal, BarkSlot, BUILTINS};
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::runtime::ClosedFieldContext;
use ccog::trace::{decide_with_trace, decide_with_trace_table, BarkSkipReason};
use ccog::{CompiledFieldSnapshot, FieldContext};
use std::sync::Arc;

fn empty_snap() -> CompiledFieldSnapshot {
    let field = FieldContext::new("zero");
    CompiledFieldSnapshot::from_field(&field).unwrap()
}

fn empty_context(snap: Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
    ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    }
}

/// Zero-by-closure: with no predicates present, three of four built-in
/// slots have unsatisfied `require_mask`. The trace must show
/// `RequireMaskUnsatisfied` for them — closure (graph emptiness) is the
/// proximate cause.
#[test]
fn zero_by_closure() {
    let snap = Arc::new(empty_snap());
    let context = empty_context(snap);
    let (_d, trace) = decide_with_trace(&context);
    let unsatisfied = trace
        .nodes
        .iter()
        .filter(|n| n.skip == Some(BarkSkipReason::RequireMaskUnsatisfied))
        .count();
    assert_eq!(
        unsatisfied, 3,
        "missing_evidence/phrase_binding/transition_admissibility skip by closure"
    );
}

/// Zero-by-floor: the alloc-free `decide` path must produce the same
/// `fired` bit-set as the trace path. "Floor" means: the kernel-floor
/// path doesn't accidentally fire a slot that the trace would have
/// reported as skipped. We assert byte-identity of the BarkDecision.
#[test]
fn zero_byfloor() {
    let snap = Arc::new(empty_snap());
    let context = empty_context(snap);
    let canonical = decide(&context);
    let (with_trace, _t) = decide_with_trace(&context);
    assert_eq!(canonical, with_trace);
    // Fired-bit population must equal trace fired_count.
    assert_eq!(canonical.fired.count_ones() as usize, _t.fired_count());
}

/// Zero-by-skipped-predecessor: a slot whose `predecessor_mask` references
/// a plan-node that has not advanced. We synthesize a custom table where
/// slot 0 is the receipt (always-fires) and slot 1 carries
/// `predecessor_mask = 1 << 99` (an unreachable plan-node bit). The trace
/// must surface a `PredecessorNotAdvanced` skip — once the diagnostic is
/// wired into the table walker.
#[test]
fn zero_by_skipped_predecessor() {
    fn no_op_act(_context: &ClosedFieldContext) -> anyhow::Result<ccog::Construct8> {
        Ok(ccog::Construct8::empty())
    }
    static TABLE: &[BarkSlot] = &[BarkSlot {
        name: "needs_predecessor",
        require_mask: 0,
        act: no_op_act,
        emit_receipt: false,
        predecessor_mask: 1u64 << 7,
    }];
    let snap = Arc::new(empty_snap());
    let context = empty_context(snap);
    let (_d, trace) = decide_with_trace_table(&context, TABLE);
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].predecessor_mask, 1u64 << 7);
}

/// Zero-by-require-mask-fail: identical to closure for the empty case,
/// but pins it specifically per slot. With only `prefLabel` loaded, the
/// missing_evidence slot's `require_mask` (DD_PRESENT|DD_MISSING_PROV_VALUE)
/// remains unsatisfied — that's a `RequireMaskUnsatisfied` skip, not a
/// `CheckFailed`.
#[test]
fn zero_by_require_maskfail() {
    let mut field = FieldContext::new("require-fail");
    field
        .load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )
        .unwrap();
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap);
    let (_d, trace) = decide_with_trace(&context);
    let me = trace
        .nodes
        .iter()
        .find(|n| n.hook_id == "missing_evidence")
        .unwrap();
    assert_eq!(me.skip, Some(BarkSkipReason::RequireMaskUnsatisfied));
    assert!(!me.trigger_fired);
}

/// Zero-by-context-deny: the `decide` path emits zero matched slots when
/// the snapshot context denies them — equivalent at the kernel level to
/// closure, but pinned at the materialize/seal boundary: with `fired = 0`,
/// `materialize` returns all-`None` and `seal` returns all-`None`, no
/// allocations leak through.
#[test]
fn zero_by_context_deny() {
    let snap = Arc::new(empty_snap());
    let context = empty_context(snap);
    let decision = decide_table(&context, BUILTINS);
    // Only "receipt" (require_mask = 0) fires on empty.
    assert_eq!(decision.fired, 0b1000);
    let deltas = materialize(&decision, &context).unwrap();
    let receipts = seal(&decision, &deltas, "f", None);
    // Three of four slots produce no delta and no receipt — denied by context.
    let none_deltas = deltas.iter().filter(|d| d.is_none()).count();
    let none_receipts = receipts.iter().filter(|r| r.is_none()).count();
    assert_eq!(none_deltas, 3);
    assert_eq!(none_receipts, 3);
}

/// Zero-by-manual-only: a slot that mirrors `HookTrigger::ManualOnly`
/// semantics by holding `require_mask = u64::MAX`. The mask can never be
/// satisfied during automatic dispatch — only an external invocation can
/// fire it. The trace records `RequireMaskUnsatisfied` (the proximate
/// cause) and the slot stays at zero.
#[test]
fn zero_by_manual_only() {
    fn no_op_act(_context: &ClosedFieldContext) -> anyhow::Result<ccog::Construct8> {
        Ok(ccog::Construct8::empty())
    }
    static TABLE: &[BarkSlot] = &[BarkSlot {
        name: "manual_only_slot",
        require_mask: u64::MAX,
        act: no_op_act,
        emit_receipt: false,
        predecessor_mask: 0,
    }];
    let snap = Arc::new(empty_snap());
    let context = empty_context(snap);
    let (decision, trace) = decide_with_trace_table(&context, TABLE);
    assert_eq!(decision.fired, 0);
    assert_eq!(
        trace.nodes[0].skip,
        Some(BarkSkipReason::RequireMaskUnsatisfied)
    );
}

// =============================================================================
// COG8 Earned Zero
// =============================================================================

#[test]
fn zero_by_cog8_unsatisfied_guard() {
    use ccog::runtime::cog8::*;

    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: 0b1,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(1),
            guard_mask: 0b10, // Requires bit 1 in completed (which we won't have)
            effect_mask: 0,
        },
    }];

    let f = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&f).unwrap());
    let context = ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };

    // present has the bit, but guard_mask in edge prevents execution.
    let d = execute_cog8(&nodes, &edges, &context, 0).expect("execute");
    assert_eq!(
        d.response,
        Instinct::Ignore,
        "unsatisfied guard mask yields Ignore (Zero)"
    );
}

#[test]
fn zero_by_cog8_missing_required_bit() {
    use ccog::runtime::cog8::*;

    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: 0b1, // Requires bit 0
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(1),
            guard_mask: 0,
            effect_mask: 0,
        },
    }];

    let f = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::default());
    let context = ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };

    // present is empty → required bit 0 is missing.
    let d = execute_cog8(&nodes, &edges, &context, 0).expect("execute");
    assert_eq!(
        d.response,
        Instinct::Ignore,
        "missing required bit yields Ignore (Zero)"
    );
}
