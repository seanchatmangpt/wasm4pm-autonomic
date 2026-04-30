//! Phase 1 — Petri-net motif discovery.
//!
//! A minimal, deterministic α-style discovery: from event sequences we
//! infer the directly-follows graph, derive `(activity, activity)`
//! transitions, and expose a `PetriNet` with named places and transitions.
//! This is the structural input for Phase 2 alignment.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

/// One trace = ordered list of activity names.
pub type ActivityTrace = Vec<String>;

/// Directly-follows graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dfg {
    /// Activity → activity edge counts. Deterministic insertion order.
    pub edges: IndexMap<(String, String), u32>,
    /// All activities seen, in first-seen order.
    pub activities: IndexSet<String>,
    /// Activities that started a trace at least once.
    pub starts: IndexSet<String>,
    /// Activities that ended a trace at least once.
    pub ends: IndexSet<String>,
}

/// Petri net (α-style projection) — places are pre/post sets.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PetriNet {
    /// Transition names in deterministic order.
    pub transitions: Vec<String>,
    /// `from → to` arcs lifted from the DFG.
    pub arcs: Vec<(String, String)>,
    /// Initial-marking transitions (sources).
    pub initial: Vec<String>,
    /// Final-marking transitions (sinks).
    pub final_: Vec<String>,
}

/// Build a directly-follows graph from a multiset of activity traces.
#[must_use]
pub fn discover_dfg(traces: &[ActivityTrace]) -> Dfg {
    let mut dfg = Dfg::default();
    for t in traces {
        if let Some(first) = t.first() {
            dfg.starts.insert(first.clone());
        }
        if let Some(last) = t.last() {
            dfg.ends.insert(last.clone());
        }
        for a in t {
            dfg.activities.insert(a.clone());
        }
        for w in t.windows(2) {
            *dfg.edges.entry((w[0].clone(), w[1].clone())).or_insert(0) += 1;
        }
    }
    dfg
}

/// Lift a DFG into a structural Petri net (α-style projection).
#[must_use]
pub fn dfg_to_petri(dfg: &Dfg) -> PetriNet {
    let mut net = PetriNet::default();
    net.transitions = dfg.activities.iter().cloned().collect();
    net.arcs = dfg.edges.keys().cloned().collect();
    net.initial = dfg.starts.iter().cloned().collect();
    net.final_ = dfg.ends.iter().cloned().collect();
    net
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &[&str]) -> ActivityTrace {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn dfg_counts_edges_and_endpoints() {
        let log = vec![t(&["a", "b", "c"]), t(&["a", "b", "d"]), t(&["a", "c"])];
        let dfg = discover_dfg(&log);
        assert_eq!(dfg.edges[&("a".into(), "b".into())], 2);
        assert_eq!(dfg.edges[&("b".into(), "c".into())], 1);
        assert_eq!(dfg.edges[&("a".into(), "c".into())], 1);
        assert!(dfg.starts.contains("a"));
        assert!(dfg.ends.contains("c"));
        assert!(dfg.ends.contains("d"));
    }

    #[test]
    fn dfg_is_deterministic() {
        let log = vec![t(&["a", "b"]), t(&["b", "c"])];
        let d1 = discover_dfg(&log);
        let d2 = discover_dfg(&log);
        assert_eq!(d1, d2);
    }

    #[test]
    fn petri_net_lifted_from_dfg() {
        let log = vec![t(&["a", "b", "c"])];
        let dfg = discover_dfg(&log);
        let net = dfg_to_petri(&dfg);
        assert_eq!(net.transitions, vec!["a", "b", "c"]);
        assert_eq!(net.arcs, vec![("a".into(), "b".into()), ("b".into(), "c".into())]);
        assert_eq!(net.initial, vec!["a"]);
        assert_eq!(net.final_, vec!["c"]);
    }
}
