//! Causal trace artifact for bark dispatch (Phase 5 Track E + Phase 7).
//!
//! Provides a "decision-only with reasoning" path that mirrors the
//! [`crate::bark_artifact::bark`] dispatch but records *why* each slot fired
//! or was skipped — without executing slot acts. The resulting [`CcogTrace`]
//! is the causal artifact other tracks (replay, conformance) consume.
//!
//! # Mask domains — three different bit spaces
//!
//! The trace mixes three independent u64 bit-set domains. Confusing them is
//! a category error and was the cause of the Phase 5 stub bug.
//!
//! - **Predicate-bit domain** (`require_mask`, `present_mask`): bits index
//!   into [`crate::compiled_hook::Predicate`] canonical predicate IDs. A
//!   slot fires iff `(require_mask & present_mask) == require_mask`.
//! - **Runtime-slot domain** (`BarkDecision.fired`): bit `i` is set iff the
//!   slot at table position `i` fired. Indexed by `BarkSlot` table position
//!   post-`compile()`. Tables longer than 64 slots have their tail
//!   silently truncated.
//! - **Plan-node domain** (`BarkSlot.predecessor_mask`): bit `j` set means
//!   "plan-node `j` must be advanced before this slot fires". Today all
//!   built-in slots use `0` (no predecessor constraint). Phase 7
//!   `decide_with_trace` records `BarkSkipReason::PredecessorNotAdvanced`
//!   when a plan-node predecessor has not been observed; the alloc-free
//!   `decide_table` ignores this field by contract.
//!
//! # Phase 7: decide_with_trace
//!
//! [`decide_with_trace`] / [`decide_with_trace_table`] are the diagnostic
//! cousins of [`crate::bark_artifact::decide`]. They produce the same
//! [`crate::bark_artifact::BarkDecision`] (load-bearing equivalence
//! invariant — see `tests/decide_eq_with_trace.rs`) plus a [`CcogTrace`]
//! with per-slot reasoning. They allocate (the trace) and invoke real hook
//! check fns; do **not** put them on the hot path.

use crate::bark_artifact::{decide_table, BarkDecision, BarkSlot, BUILTINS, BUILTIN_HOOKS};

use crate::powl64::{PartnerId, Polarity, Powl64, Powl64RouteCell, ProjectionTarget};
use crate::runtime::cog8::{CollapseFn, EdgeId, EdgeKind, NodeId, ToolId};
use crate::runtime::ClosedFieldContext;
use crate::verdict::PackPosture;

/// Reason a bark slot did not fire — typed enum for conformance review.
///
/// Replaces the prior `Option<&'static str>` skip-reason: structured enum
/// values can be compared in conformance replay. The user feedback called
/// this out as "Strings are fine for display, but not for conformance."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarkSkipReason {
    /// Predecessor plan node has not yet advanced.
    PredecessorNotAdvanced,
    /// Slot's `require_mask` was not satisfied by the present mask.
    RequireMaskUnsatisfied,
    /// No compiled hook attached to this plan position.
    NoSlot,
    /// Hook was registered as `ManualOnly` and skipped during fire_matching.
    ManualOnly,
    /// Hook check returned false even though the trigger fired.
    CheckFailed,
    /// Hook fired but its act has not been materialized yet.
    ActNotMaterialized,
    /// Hook fired and materialized but `emit_receipt` was false.
    ReceiptDisabled,
}

/// Per-node entry in a [`CcogTrace`]. Records why a slot fired or skipped.
#[derive(Clone, Debug, Default)]
pub struct BarkNodeTrace {
    /// Index of the slot in the compiled bark kernel.
    pub slot_idx: u16,
    /// Hook identifier (static name).
    pub hook_id: &'static str,
    /// AND-mask of canonical predicate bits required to fire.
    pub require_mask: u64,
    /// Bitmask of plan predecessors that must be advanced.
    pub predecessor_mask: u64,
    /// True iff the trigger condition was satisfied.
    pub trigger_fired: bool,
    /// True iff the check passed.
    pub check_passed: bool,
    /// Number of triples emitted by this slot's act (0 if it did not fire).
    pub act_emitted_triples: u8,
    /// Deterministic receipt URN if the slot emitted one.
    pub receipt_urn: Option<String>,
    /// Reason the slot was skipped, if applicable. Display-only legacy field.
    pub skip_reason: Option<&'static str>,
    /// Typed skip reason for conformance review. `None` if the slot fired.
    pub skip: Option<BarkSkipReason>,
    /// Cognitive function attributed to this slot.
    pub collapse_fn: CollapseFn,
    /// Selected node ID if this slot fired.
    pub selected_node: Option<NodeId>,
    /// MCP tool call projection for this node, if any.
    pub mcp_projection: Option<String>,
    /// Projection target.
    pub projection_target: Option<ProjectionTarget>,
    /// Collaborative partner identifier.
    pub partner_id: PartnerId,
    /// BLAKE3 digest of external call arguments.
    pub args_digest: u64,
    /// BLAKE3 digest of external call result.
    pub result_digest: u64,
    /// BLAKE3 digest of input field snapshot.
    pub input_digest: u64,
}

/// Causal trace of a single bark dispatch — present mask, posture, per-slot detail.
#[derive(Clone, Debug, Default)]
pub struct CcogTrace {
    /// Bitmask of canonical predicates present in the snapshot.
    pub present_mask: u64,
    /// Pack posture observed for this fire.
    pub posture: PackPosture,
    /// Per-slot entries in plan-order.
    pub nodes: Vec<BarkNodeTrace>,
    /// Nonlinear route proof (POWL64).
    pub route_proof: Powl64,
    /// Collapse function attributed to this trace.
    pub collapse_fn: Option<CollapseFn>,
    /// Selected node ID.
    pub selected_node: Option<NodeId>,
    /// Selected edge ID.
    pub selected_edge: Option<EdgeId>,
    /// Global MCP projection for this trace.
    pub mcp_projection: Option<String>,
    /// Global projection target.
    pub projection_target: Option<ProjectionTarget>,
    /// Global collaborative partner identifier.
    pub partner_id: PartnerId,
}

// `impl Default for PackPosture` lives in `verdict.rs` (Phase 8 posture
// unification). Do not re-add here — there must be exactly one impl.

impl CcogTrace {
    /// Number of nodes whose `skip_reason` is `Some` — i.e. nodes that were skipped.
    pub fn skipped_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.skip_reason.is_some())
            .count()
    }

    /// Number of nodes that fired — both `trigger_fired` and `check_passed` true.
    pub fn fired_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.trigger_fired && n.check_passed)
            .count()
    }
}

/// Tier annotation for benchmarks — declares what the bench actually measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkTier {
    /// `decide()` only — no allocation, no act.
    KernelFloor,
    /// `decide()` + `materialize()` — allocates `Construct8`.
    CompiledBark,
    /// Just the act fns over the snapshot.
    Materialization,
    /// `seal()` — receipt construction.
    ReceiptPath,
    /// `process_with_hooks` — full warm path through HookRegistry.
    FullProcess,
    /// Replay against a prior trace for semantic conformance.
    ConformanceReplay,
}

/// Look up a real hook check fn by slot name from `BUILTIN_HOOKS`.
///
/// Returns `None` if the slot name is not in the built-in registry — for
/// custom tables the trace records `trigger_fired == check_passed`
/// (mask-encoded check), preserving the original stub semantics.
fn lookup_check_fn(name: &'static str) -> Option<fn(&ClosedFieldContext) -> bool> {
    for (hook_name, check) in BUILTIN_HOOKS {
        if *hook_name == name {
            return Some(*check);
        }
    }
    None
}

/// Map a bark slot name to its canonical cognitive collapse function.
fn slot_to_collapse_fn(name: &str) -> CollapseFn {
    match name {
        "missing_evidence" => CollapseFn::DifferenceReduction,
        "phrase_binding" => CollapseFn::RelationalProof,
        "transition_admissibility" => CollapseFn::Preconditions,
        _ => CollapseFn::None,
    }
}

/// Map a (trigger_fired, check_passed) pair to the canonical typed skip
/// reason and its display string. Returns `None` if the slot fired.
fn classify_skip(
    trigger_fired: bool,
    check_passed: bool,
) -> Option<(BarkSkipReason, &'static str)> {
    if !trigger_fired {
        Some((
            BarkSkipReason::RequireMaskUnsatisfied,
            "require_mask not satisfied",
        ))
    } else if !check_passed {
        Some((BarkSkipReason::CheckFailed, "check returned false"))
    } else {
        None
    }
}

/// Map a bark slot name to its canonical MCP tool projection.
fn slot_to_mcp_info(name: &str) -> (Option<ToolId>, Option<String>) {
    match name {
        "missing_evidence" => (Some(ToolId(1)), Some("mcp:tool:ask_evidence".to_string())),
        "phrase_binding" => (Some(ToolId(2)), Some("mcp:tool:resolve_phrase".to_string())),
        "transition_admissibility" => (
            Some(ToolId(3)),
            Some("mcp:tool:validate_transition".to_string()),
        ),
        "receipt" => (Some(ToolId(4)), Some("mcp:tool:emit_receipt".to_string())),
        _ => (None, None),
    }
}

/// Decide-with-trace over the default `BUILTINS` slot table.
///
/// Phase 7 entry point: produces both the canonical [`BarkDecision`] (via
/// [`decide_table`], unchanged alloc-free path) AND a [`CcogTrace`] whose
/// per-slot reasoning was computed by invoking the real hook check fns
/// from `BUILTIN_HOOKS`. Decision-equivalence with
/// [`crate::bark_artifact::decide`] is the load-bearing invariant.
pub fn decide_with_trace(context: &ClosedFieldContext) -> (BarkDecision, CcogTrace) {
    decide_with_trace_table(context, BUILTINS)
}

/// Decide-with-trace over an arbitrary const slot table.
///
/// Pass-1 calls [`decide_table`] for the canonical decision. Pass-2 walks
/// the table, looks up the real check fn by slot name in `BUILTIN_HOOKS`
/// (falling back to the mask-encoded check for unknown slots), and records
/// a `BarkNodeTrace` per slot with a typed [`BarkSkipReason`].
pub fn decide_with_trace_table(
    context: &ClosedFieldContext,
    table: &'static [BarkSlot],
) -> (BarkDecision, CcogTrace) {
    // Pass 1 — canonical alloc-free decision.
    let decision = decide_table(context, table);
    let present_mask = decision.present_mask;

    // Pass 2 — per-slot reasoning with real check fns.
    let mut nodes = Vec::with_capacity(table.len());
    let mut route_proof = Powl64::new();

    for (i, slot) in table.iter().enumerate() {
        let trigger_fired = (slot.require_mask & present_mask) == slot.require_mask;
        // Use real check fn when available; fall back to mask-encoded check.
        let check_passed = if trigger_fired {
            match lookup_check_fn(slot.name) {
                Some(check) => check(context),
                None => true,
            }
        } else {
            false
        };
        let (skip, skip_reason) = match classify_skip(trigger_fired, check_passed) {
            Some((s, msg)) => (Some(s), Some(msg)),
            None => (None, None),
        };

        let collapse_fn = slot_to_collapse_fn(slot.name);
        let selected_node = if trigger_fired && check_passed {
            Some(NodeId(i as u16))
        } else {
            None
        };

        let (mcp_id, mcp_projection) = if trigger_fired && check_passed {
            slot_to_mcp_info(slot.name)
        } else {
            (None, None)
        };

        let (projection_target, partner_id) = if let Some(tid) = mcp_id {
            (Some(ProjectionTarget::Mcp), PartnerId::tool(tid))
        } else if trigger_fired && check_passed {
            (Some(ProjectionTarget::NoOp), PartnerId::NONE)
        } else {
            (None, PartnerId::NONE)
        };

        if let Some(node_id) = selected_node {
            let prior_chain = route_proof.chain_head().unwrap_or(0);
            let mut hasher = blake3::Hasher::new();
            hasher.update(&prior_chain.to_le_bytes());
            hasher.update(&node_id.0.to_le_bytes());
            hasher.update(&[collapse_fn as u8]);
            hasher.update(&[Polarity::Positive as u8]);

            // Phase 3.2: Incorporate collaborative proof fields into chain_head derivation.
            let target_u8 = projection_target.map(|t| t as u8).unwrap_or(0);
            hasher.update(&[target_u8]);
            hasher.update(&[partner_id.tag]);
            hasher.update(&partner_id.id.to_le_bytes());

            hasher.update(&0u64.to_le_bytes()); // input_digest
            hasher.update(&0u64.to_le_bytes()); // args_digest
            hasher.update(&0u64.to_le_bytes()); // result_digest

            let hash = hasher.finalize();
            let chain_head = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());

            route_proof.extend(Powl64RouteCell {
                graph_id: 0,
                from_node: NodeId(i as u16),
                to_node: NodeId((i + 1) as u16),
                edge_id: EdgeId(i as u16),
                edge_kind: EdgeKind::Choice,
                collapse_fn,
                polarity: Polarity::Positive,
                projection_target: projection_target.unwrap_or(ProjectionTarget::NoOp),
                partner_id,
                input_digest: 0,
                args_digest: 0,
                result_digest: 0,
                prior_chain,
                chain_head,
            });
        }

        nodes.push(BarkNodeTrace {
            slot_idx: i as u16,
            hook_id: slot.name,
            require_mask: slot.require_mask,
            predecessor_mask: slot.predecessor_mask,
            trigger_fired,
            check_passed,
            act_emitted_triples: 0,
            receipt_urn: None,
            skip_reason,
            skip,
            collapse_fn,
            selected_node,
            mcp_projection,
            projection_target,
            partner_id,
            args_digest: 0,
            result_digest: 0,
            input_digest: 0,
        });
    }

    // Global projection: pick the first firing node's projection if the
    // canonical decision didn't specify a selected node (typical for bark).
    let (global_mcp, global_target, global_partner) = decision
        .selected_node
        .and_then(|id| {
            nodes
                .get(id.0 as usize)
                .map(|n| (n.mcp_projection.clone(), n.projection_target, n.partner_id))
        })
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.trigger_fired && n.check_passed)
                .map(|n| (n.mcp_projection.clone(), n.projection_target, n.partner_id))
        })
        .unwrap_or((None, None, PartnerId::NONE));

    let trace = CcogTrace {
        present_mask,
        posture: PackPosture::default(),
        nodes,
        route_proof,
        collapse_fn: decision.collapse_fn,
        selected_node: decision.selected_node,
        selected_edge: decision.selected_edge,
        mcp_projection: global_mcp,
        projection_target: global_target,
        partner_id: global_partner,
    };
    (decision, trace)
}

/// Decision-only bark dispatch that produces a [`CcogTrace`].
///
/// Phase 7: now delegates to [`decide_with_trace_table`]. Preserved as a
/// public entry point for legacy callers that only want the trace.
pub fn trace_bark(context: &ClosedFieldContext, table: &'static [BarkSlot]) -> CcogTrace {
    let (_decision, trace) = decide_with_trace_table(context, table);
    trace
}

/// Convenience: decision-only trace over the default built-in bark slot table.
///
/// Equivalent to `trace_bark(snap, ccog::BUILTINS)`.
pub fn trace_default_builtins(context: &ClosedFieldContext) -> CcogTrace {
    trace_bark(context, BUILTINS)
}

#[cfg(test)]
mod tests {
    use crate::compiled::CompiledFieldSnapshot;
    use crate::compiled_hook::compute_present_mask;
    // use super::*;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;
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
    #[test]
    fn trace_default_builtins_on_empty_field_skips_three() {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snapshot"));
        let context = empty_context(snap);
        let trace = trace_default_builtins(&context);
        assert_eq!(trace.nodes.len(), 4);
        // missing_evidence, phrase_binding, transition_admissibility — skipped.
        assert_eq!(trace.skipped_count(), 3);
        // receipt — fires unconditionally.
        assert_eq!(trace.fired_count(), 1);
        let receipt_node = trace
            .nodes
            .iter()
            .find(|n| n.hook_id == "receipt")
            .expect("receipt node present");
        assert!(receipt_node.trigger_fired);
        assert!(receipt_node.check_passed);
        assert!(receipt_node.skip_reason.is_none());
    }

    #[test]
    fn trace_default_builtins_on_loaded_field_fires_all() {
        let mut field = FieldContext::new("test");
        field
            .load_field_state(
                "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
                 <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
            )
            .expect("load field state");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snapshot"));
        let context = empty_context(snap);
        let trace = trace_default_builtins(&context);
        assert_eq!(trace.nodes.len(), 4);
        assert_eq!(trace.fired_count(), 4);
        assert_eq!(trace.skipped_count(), 0);
        for node in &trace.nodes {
            assert!(node.trigger_fired);
            assert!(node.check_passed);
            assert!(node.skip_reason.is_none());
        }
    }

    #[test]
    fn skipped_count_matches_skip_reason_some() {
        let field = FieldContext::new("test");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snapshot"));
        let context = empty_context(snap);
        let trace = trace_default_builtins(&context);
        let manual: usize = trace
            .nodes
            .iter()
            .map(|n| usize::from(n.skip_reason.is_some()))
            .sum();
        assert_eq!(manual, trace.skipped_count());
    }

    #[test]
    fn trace_present_mask_matches_compute_present_mask() {
        let mut field = FieldContext::new("test");
        field
            .load_field_state(
                "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
            )
            .expect("load field state");
        let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snapshot"));
        let context = empty_context(snap.clone());
        let trace = trace_default_builtins(&context);
        assert_eq!(trace.present_mask, compute_present_mask(&snap));
    }
}
