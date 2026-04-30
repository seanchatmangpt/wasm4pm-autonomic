//! Conformance replay (Phase 7).
//!
//! Re-runs [`crate::trace::decide_with_trace_table`] against a snapshot and
//! a table, then diffs the produced [`crate::trace::CcogTrace`] against a
//! prior trace. Emits a [`ReplayReport`] that names the first divergent
//! slot and its skip reasons. Replay is the contract that `decide_with_trace`
//! is reproducible — same snapshot, same table, same decision, same
//! per-slot reasoning.

use crate::bark_artifact::BarkSlot;
use crate::compiled::CompiledFieldSnapshot;
use crate::trace::{decide_with_trace_table, BarkSkipReason, CcogTrace};

/// Result of [`replay_trace`] — pinpoints the first divergence, if any.
///
/// `decision_eq` is `true` iff every slot's `(trigger_fired, check_passed,
/// skip)` triple matches between the original trace and the replay. On the
/// first divergence, `diverged_slot` is set and `original_skip` /
/// `replay_skip` capture the typed reasons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayReport {
    /// True iff every per-slot record is identical.
    pub decision_eq: bool,
    /// Slot index where the first divergence occurred, if any.
    pub diverged_slot: Option<u16>,
    /// Skip reason in the original trace at the divergence point.
    pub original_skip: Option<BarkSkipReason>,
    /// Skip reason in the replay trace at the divergence point.
    pub replay_skip: Option<BarkSkipReason>,
}

/// Replay a trace against a snapshot and table.
///
/// Re-invokes [`decide_with_trace_table`] and walks both traces in
/// lock-step. The first slot whose `(trigger_fired, check_passed, skip)`
/// differs is reported. If both traces agree on every slot,
/// `decision_eq = true` and `diverged_slot = None`.
pub fn replay_trace(
    trace: &CcogTrace,
    snap: &CompiledFieldSnapshot,
    table: &'static [BarkSlot],
) -> ReplayReport {
    let (_decision, replay) = decide_with_trace_table(snap, table);

    if trace.nodes.len() != replay.nodes.len() {
        let idx = trace.nodes.len().min(replay.nodes.len()) as u16;
        return ReplayReport {
            decision_eq: false,
            diverged_slot: Some(idx),
            original_skip: trace.nodes.get(idx as usize).and_then(|n| n.skip),
            replay_skip: replay.nodes.get(idx as usize).and_then(|n| n.skip),
        };
    }

    for (i, (a, b)) in trace.nodes.iter().zip(replay.nodes.iter()).enumerate() {
        if a.trigger_fired != b.trigger_fired
            || a.check_passed != b.check_passed
            || a.skip != b.skip
        {
            return ReplayReport {
                decision_eq: false,
                diverged_slot: Some(i as u16),
                original_skip: a.skip,
                replay_skip: b.skip,
            };
        }
    }

    ReplayReport {
        decision_eq: true,
        diverged_slot: None,
        original_skip: None,
        replay_skip: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bark_artifact::BUILTINS;
    use crate::field::FieldContext;
    use crate::trace::decide_with_trace;

    #[test]
    fn replay_matches_self_on_empty_field() {
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
        let (_d, trace) = decide_with_trace(&snap);
        let report = replay_trace(&trace, &snap, BUILTINS);
        assert!(report.decision_eq);
        assert!(report.diverged_slot.is_none());
    }

    #[test]
    fn replay_matches_self_on_loaded_field() {
        let mut field = FieldContext::new("test");
        field
            .load_field_state(
                "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
            )
            .unwrap();
        let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
        let (_d, trace) = decide_with_trace(&snap);
        let report = replay_trace(&trace, &snap, BUILTINS);
        assert!(report.decision_eq, "{:?}", report);
    }
}
