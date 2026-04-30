//! Causal trace artifact for bark dispatch (Phase 5 Track E).
//!
//! Provides a "decision-only with reasoning" path that mirrors the
//! [`crate::bark_artifact::bark`] dispatch but records *why* each slot fired
//! or was skipped — without executing slot acts. The resulting [`CcogTrace`]
//! is the causal artifact other tracks (replay, conformance) consume.

use crate::bark_artifact::{BarkSlot, BUILTINS};
use crate::compiled::CompiledFieldSnapshot;
use crate::compiled_hook::compute_present_mask;
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
}

impl Default for PackPosture {
    fn default() -> Self {
        PackPosture::Calm
    }
}

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

/// Decision-only bark dispatch that produces a [`CcogTrace`].
///
/// Re-implements the same `present_mask` / `require_mask` walk as
/// [`crate::bark_artifact::bark_table`] but records the per-slot reasoning
/// instead of executing the slot's `act` function. Useful for replay,
/// conformance checking, and debugging — not for materialization.
pub fn trace_bark(snap: &CompiledFieldSnapshot, table: &'static [BarkSlot]) -> CcogTrace {
    let present_mask = compute_present_mask(snap);
    let mut trace = CcogTrace {
        present_mask,
        posture: PackPosture::default(),
        nodes: Vec::with_capacity(table.len()),
    };
    for (i, slot) in table.iter().enumerate() {
        let trigger_fired = (slot.require_mask & present_mask) == slot.require_mask;
        let check_passed = trigger_fired; // mask-encoded
        let (skip_reason, skip) = if !trigger_fired {
            (
                Some("require_mask not satisfied"),
                Some(BarkSkipReason::RequireMaskUnsatisfied),
            )
        } else {
            (None, None)
        };
        let node = BarkNodeTrace {
            slot_idx: i as u16,
            hook_id: slot.name,
            require_mask: slot.require_mask,
            predecessor_mask: 0,
            trigger_fired,
            check_passed,
            act_emitted_triples: 0,
            receipt_urn: None,
            skip_reason,
            skip,
        };
        trace.nodes.push(node);
    }
    trace
}

/// Convenience: decision-only trace over the default built-in bark slot table.
///
/// Equivalent to `trace_bark(snap, ccog::BUILTINS)`.
pub fn trace_default_builtins(snap: &CompiledFieldSnapshot) -> CcogTrace {
    trace_bark(snap, BUILTINS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;

    #[test]
    fn trace_default_builtins_on_empty_field_skips_three() {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snapshot");
        let trace = trace_default_builtins(&snap);
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
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snapshot");
        let trace = trace_default_builtins(&snap);
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
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snapshot");
        let trace = trace_default_builtins(&snap);
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
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snapshot");
        let trace = trace_default_builtins(&snap);
        assert_eq!(trace.present_mask, compute_present_mask(&snap));
    }
}
