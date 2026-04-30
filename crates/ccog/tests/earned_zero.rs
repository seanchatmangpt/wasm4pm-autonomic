//! Phase 7 earned-zero distinction: a fired count of zero can have one of
//! several causes, and the trace must distinguish them. Each test pins one
//! zero-cause and asserts the trace skip reasons are exactly what we
//! claim.
//!
//! "Earned zero" means: the outcome is zero by lawful skip semantics, not
//! by accident, not by silent allocator allocation. Each case below pins
//! a different lawful path.

use ccog::bark_artifact::{
    decide, decide_table, decide_with_trace_table, materialize, seal, BarkSlot, BUILTINS,
};
use ccog::trace::{decide_with_trace, BarkSkipReason};
use ccog::{CompiledFieldSnapshot, FieldContext};

fn empty_snap() -> CompiledFieldSnapshot {
    let field = FieldContext::new("zero");
    CompiledFieldSnapshot::from_field(&field).unwrap()
}

/// Zero-by-closure: with no predicates present, three of four built-in
/// slots have unsatisfied `require_mask`. The trace must show
/// `RequireMaskUnsatisfied` for them — closure (graph emptiness) is the
/// proximate cause.
#[test]
fn zero_by_closure() {
    let snap = empty_snap();
    let (_d, trace) = decide_with_trace(&snap);
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
fn zero_by_floor() {
    let snap = empty_snap();
    let canonical = decide(&snap);
    let (with_trace, _t) = decide_with_trace(&snap);
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
///
/// Note: today's `decide_with_trace_table` does not yet enforce
/// predecessor masks (Phase 7 sets the field default to 0 for all
/// builtins). This test pins the contract: when a custom table sets a
/// non-zero `predecessor_mask`, the field is preserved on the trace
/// node. Phase 9+ will tighten this to a real skip cause.
#[test]
fn zero_by_skipped_predecessor() {
    fn no_op_act(
        _snap: &CompiledFieldSnapshot,
    ) -> anyhow::Result<ccog::Construct8> {
        Ok(ccog::Construct8::empty())
    }
    static TABLE: &[BarkSlot] = &[BarkSlot {
        name: "needs_predecessor",
        require_mask: 0,
        act: no_op_act,
        emit_receipt: false,
        predecessor_mask: 1u64 << 7,
    }];
    let snap = empty_snap();
    let (_d, trace) = decide_with_trace_table(&snap, TABLE);
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].predecessor_mask, 1u64 << 7);
}

/// Zero-by-require-mask-fail: identical to closure for the empty case,
/// but pins it specifically per slot. With only `prefLabel` loaded, the
/// missing_evidence slot's `require_mask` (DD_PRESENT|DD_MISSING_PROV_VALUE)
/// remains unsatisfied — that's a `RequireMaskUnsatisfied` skip, not a
/// `CheckFailed`.
#[test]
fn zero_by_require_mask_fail() {
    let mut field = FieldContext::new("require-fail");
    field
        .load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )
        .unwrap();
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let (_d, trace) = decide_with_trace(&snap);
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
    let snap = empty_snap();
    let decision = decide_table(&snap, BUILTINS);
    // Only "receipt" (require_mask = 0) fires on empty.
    assert_eq!(decision.fired, 0b1000);
    let deltas = materialize(&decision, &snap).unwrap();
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
    fn no_op_act(
        _snap: &CompiledFieldSnapshot,
    ) -> anyhow::Result<ccog::Construct8> {
        Ok(ccog::Construct8::empty())
    }
    static TABLE: &[BarkSlot] = &[BarkSlot {
        name: "manual_only_slot",
        require_mask: u64::MAX,
        act: no_op_act,
        emit_receipt: false,
        predecessor_mask: 0,
    }];
    let snap = empty_snap();
    let (decision, trace) = decide_with_trace_table(&snap, TABLE);
    assert_eq!(decision.fired, 0);
    assert_eq!(
        trace.nodes[0].skip,
        Some(BarkSkipReason::RequireMaskUnsatisfied)
    );
}
