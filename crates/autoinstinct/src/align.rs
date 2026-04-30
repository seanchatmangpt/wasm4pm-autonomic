//! Phase 2 — Conformance alignment.
//!
//! Token-replay-style alignment between a `PetriNet` and an activity trace.
//! Returns per-trace fitness in [0, 1]; `1.0` is exact replay, `0.0` is
//! every step a deviation. The gauntlet uses this as an admission surface.

use serde::{Deserialize, Serialize};

use crate::petri::{ActivityTrace, PetriNet};

/// Alignment outcome for a single trace.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TraceFitness {
    /// Number of steps that replayed cleanly against the net.
    pub matched: u32,
    /// Number of steps that deviated.
    pub deviations: u32,
    /// `matched / (matched + deviations)`. NaN-safe (returns 1.0 for empty).
    pub fitness: f64,
}

/// Alignment outcome for an entire log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignmentReport {
    /// Per-trace fitnesses in input order.
    pub per_trace: Vec<TraceFitness>,
    /// Mean fitness across the log.
    pub mean_fitness: f64,
}

/// Replay one trace against a Petri net.
///
/// Step matches iff the net contains an arc `(prev → curr)` in `arcs`
/// (or `prev` is `None` and `curr` is in `initial`). All other steps
/// count as deviations.
#[must_use]
pub fn replay(trace: &ActivityTrace, net: &PetriNet) -> TraceFitness {
    let mut matched = 0u32;
    let mut deviations = 0u32;
    let mut prev: Option<&str> = None;
    for step in trace {
        let ok = match prev {
            None => net.initial.iter().any(|s| s == step),
            Some(p) => net.arcs.iter().any(|(a, b)| a == p && b == step),
        };
        if ok {
            matched += 1;
        } else {
            deviations += 1;
        }
        prev = Some(step.as_str());
    }
    let fitness = if matched + deviations == 0 {
        1.0
    } else {
        matched as f64 / (matched + deviations) as f64
    };
    TraceFitness {
        matched,
        deviations,
        fitness,
    }
}

/// Replay an entire log; aggregate the mean fitness.
#[must_use]
pub fn align_log(log: &[ActivityTrace], net: &PetriNet) -> AlignmentReport {
    let per_trace: Vec<TraceFitness> = log.iter().map(|t| replay(t, net)).collect();
    let mean_fitness = if per_trace.is_empty() {
        1.0
    } else {
        per_trace.iter().map(|t| t.fitness).sum::<f64>() / per_trace.len() as f64
    };
    AlignmentReport {
        per_trace,
        mean_fitness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petri::{dfg_to_petri, discover_dfg};

    fn t(s: &[&str]) -> ActivityTrace {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn exact_replay_yields_fitness_one() {
        let log = vec![t(&["a", "b", "c"])];
        let net = dfg_to_petri(&discover_dfg(&log));
        let r = align_log(&log, &net);
        assert!((r.mean_fitness - 1.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_trace_drops_fitness() {
        let train = vec![t(&["a", "b", "c"])];
        let net = dfg_to_petri(&discover_dfg(&train));
        let probe = vec![t(&["x", "y"])];
        let r = align_log(&probe, &net);
        assert!(r.mean_fitness < 0.5, "{}", r.mean_fitness);
    }

    #[test]
    fn fitness_is_deterministic() {
        let log = vec![t(&["a", "b"])];
        let net = dfg_to_petri(&discover_dfg(&log));
        let r1 = align_log(&log, &net);
        let r2 = align_log(&log, &net);
        assert_eq!(r1.mean_fitness, r2.mean_fitness);
    }
}
