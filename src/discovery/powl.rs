/// High-level POWL process discovery from traces.
///
/// `discover_powl(traces: &[Trace]) → PowlModel` — Alpha/Inductive Miner
/// variant that operates directly on `Trace` slices, building a DFG and
/// delegating to the bitset-based `mine_powl` kernel.
///
/// This module is the public entry point used by the spec-level interface.
/// The underlying algorithm is the Nanosecond Inductive Miner from
/// `crate::powl::discovery`.
///
/// ## Wire-in Decision (PDC 2025 Phase 2)
///
/// The `discover_powl` function is wired into the ostar_bridge JSON RPC interface
/// as the `discover_powl` operation. It accepts an EventLog and returns a PetriNet
/// converted from the discovered POWL model via `powl_to_wf_net`. This enables
/// end-to-end POWL-driven discovery pipelines in the autonomic kernel without
/// requiring Petri net serialization overhead.
use crate::models::{AttributeValue, Trace};
use crate::powl::core::{PowlModel, PowlNode};
use crate::powl::discovery::mine_powl;
use crate::utils::dense_kernel::KBitSet;

/// Maximum number of distinct activities supported.
/// WORDS=8 → 512 activities. Sufficient for any realistic process log.
pub const DISCOVER_WORDS: usize = 8;

/// Error returned when discovery fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// No activities found in the trace set (empty log).
    EmptyLog,
    /// Too many distinct activities (> DISCOVER_WORDS * 64).
    TooManyActivities { found: usize, max: usize },
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::EmptyLog => write!(f, "Cannot discover a model from an empty log"),
            DiscoveryError::TooManyActivities { found, max } => write!(
                f,
                "Log has {} distinct activities; maximum supported is {}",
                found, max
            ),
        }
    }
}

/// Extract the `concept:name` attribute from an event, if present.
#[inline]
fn activity_name(event: &crate::models::Event) -> Option<&str> {
    event
        .attributes
        .iter()
        .find(|a| a.key == "concept:name")
        .and_then(|a| {
            if let AttributeValue::String(s) = &a.value {
                Some(s.as_str())
            } else {
                None
            }
        })
}

/// Discover a POWL model from a slice of traces using the Inductive Miner algorithm.
///
/// Steps:
/// 1. Collect all distinct activity labels (assign dense integer IDs).
/// 2. Build a Directly-Follows Graph (DFG) as a `KBitSet` matrix.
/// 3. Compute the activity footprint bitmask.
/// 4. Delegate to `mine_powl` for XOR/SEQUENCE/PARALLEL/LOOP cut detection.
/// 5. Wrap the resulting `PowlNode` in a `PowlModel`.
///
/// Returns `DiscoveryError::EmptyLog` when no events are present.
pub fn discover_powl(traces: &[Trace]) -> Result<PowlModel<DISCOVER_WORDS>, DiscoveryError> {
    // --- Step 1: collect distinct activities in deterministic order ----------
    let mut seen: Vec<String> = Vec::new();
    for trace in traces {
        for event in &trace.events {
            if let Some(name) = activity_name(event) {
                if !seen.iter().any(|s| s == name) {
                    seen.push(name.to_string());
                }
            }
        }
    }

    if seen.is_empty() {
        return Err(DiscoveryError::EmptyLog);
    }

    let max_activities = DISCOVER_WORDS * 64;
    if seen.len() > max_activities {
        return Err(DiscoveryError::TooManyActivities {
            found: seen.len(),
            max: max_activities,
        });
    }

    // --- Step 2: build DFG ---------------------------------------------------
    // dfg[i] is a bitset of all activities that directly follow activity i.
    let mut dfg: Vec<KBitSet<DISCOVER_WORDS>> = vec![KBitSet::zero(); max_activities];

    let mut footprint: KBitSet<DISCOVER_WORDS> = KBitSet::zero();

    for trace in traces {
        let mut prev_idx: Option<usize> = None;

        for event in &trace.events {
            let Some(name) = activity_name(event) else {
                continue;
            };

            let idx = match seen.iter().position(|s| s == name) {
                Some(i) => i,
                None => continue,
            };

            let _ = footprint.set(idx);

            if let Some(prev) = prev_idx {
                let _ = dfg[prev].set(idx);
            }
            prev_idx = Some(idx);
        }
    }

    // --- Step 3: mine POWL ---------------------------------------------------
    let root: PowlNode = mine_powl(&dfg, footprint, &seen);

    // PowlModel::new validates structural soundness
    let model = PowlModel::new(root);

    Ok(model)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Attribute, AttributeValue, Event, Trace};
    use crate::powl::core::{PowlNode, PowlOperator};

    fn make_trace(id: &str, activities: &[&str]) -> Trace {
        Trace {
            id: id.to_string(),
            events: activities
                .iter()
                .map(|a| Event {
                    attributes: vec![Attribute {
                        key: "concept:name".to_string(),
                        value: AttributeValue::String(a.to_string()),
                    }],
                })
                .collect(),
            attributes: vec![],
        }
    }

    #[test]
    fn test_discover_sequence() {
        // Three traces all following A→B→C must produce a SEQUENCE model.
        let traces = vec![
            make_trace("c1", &["A", "B", "C"]),
            make_trace("c2", &["A", "B", "C"]),
        ];

        let model = discover_powl(&traces).expect("discover_powl must succeed");

        match &model.root {
            PowlNode::Operator { operator, children } => {
                assert_eq!(
                    *operator,
                    PowlOperator::SEQUENCE,
                    "pure sequential log must yield SEQUENCE"
                );
                assert_eq!(children.len(), 3);
            }
            other => panic!("Expected SEQUENCE, got {:?}", other),
        }
    }

    #[test]
    fn test_discover_single_activity() {
        let traces = vec![make_trace("c1", &["OnlyActivity"])];
        let model = discover_powl(&traces).expect("single-activity discover must succeed");

        match &model.root {
            PowlNode::Transition { label, .. } => {
                assert_eq!(label.as_deref(), Some("OnlyActivity"));
            }
            other => panic!("Expected Transition, got {:?}", other),
        }
    }

    #[test]
    fn test_discover_empty_log_returns_error() {
        let traces: Vec<Trace> = vec![];
        assert!(
            matches!(discover_powl(&traces), Err(DiscoveryError::EmptyLog)),
            "Expected EmptyLog error"
        );
    }

    #[test]
    fn test_discover_xor_branch() {
        // Two disjoint process paths: A→B and C→D with no overlap.
        // The DFG has two disconnected components → XOR at root.
        let traces = vec![
            make_trace("c1", &["A", "B"]),
            make_trace("c2", &["A", "B"]),
            make_trace("c3", &["C", "D"]),
            make_trace("c4", &["C", "D"]),
        ];

        let model = discover_powl(&traces).expect("discover_powl must succeed");

        match &model.root {
            PowlNode::Operator { operator, .. } => {
                assert_eq!(
                    *operator,
                    PowlOperator::XOR,
                    "disjoint paths must yield XOR"
                );
            }
            other => panic!("Expected XOR, got {:?}", other),
        }
    }

    #[test]
    fn test_discover_deterministic_result() {
        // Same log → same model hash, regardless of insertion order
        let traces_a = vec![
            make_trace("c1", &["A", "B", "C"]),
            make_trace("c2", &["A", "B", "C"]),
        ];
        let traces_b = vec![
            make_trace("c2", &["A", "B", "C"]),
            make_trace("c1", &["A", "B", "C"]),
        ];

        let m_a = discover_powl(&traces_a).expect("discover_powl must succeed");
        let m_b = discover_powl(&traces_b).expect("discover_powl must succeed");

        // Both must produce SEQUENCE of depth 3
        match (&m_a.root, &m_b.root) {
            (
                PowlNode::Operator {
                    operator: op_a,
                    children: ch_a,
                },
                PowlNode::Operator {
                    operator: op_b,
                    children: ch_b,
                },
            ) => {
                assert_eq!(op_a, op_b);
                assert_eq!(ch_a.len(), ch_b.len());
            }
            _ => panic!("Both models must be SEQUENCE operators"),
        }
    }
}
