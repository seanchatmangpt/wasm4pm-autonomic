//! GPS breed (Phase 9): means-ends gap reduction.
//!
//! GPS is admitted iff the field graph contains both at least one
//! `urn:ccog:GoalState` typed instance and at least one
//! `urn:ccog:CurrentState` typed instance. Admission is a pure
//! triple-pattern probe — no SPARQL parsing, no in-memory state.
//!
//! Materialization is **not** on the `decide()` path. It walks both
//! instance lists and emits a bounded sequence of [`ReductionMove`]s
//! pairing each goal with the lexicographically next current-state
//! subject. Bounded outputs are stored in `SmallVec<[_; 8]>` per the
//! crate-wide breed shape; residual unpaired goals are recorded in
//! `residual_gaps` (saturating at 255).

use anyhow::Result;
use oxigraph::model::NamedNode;
use smallvec::SmallVec;

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::{GapReductionPlan, ReductionMove};

const GOAL_STATE_IRI: &str = "urn:ccog:GoalState";
const CURRENT_STATE_IRI: &str = "urn:ccog:CurrentState";
const GPS_ROOT_IRI: &str = "urn:ccog:gps-root";

/// Probe whether GPS is admissible against `field`.
///
/// Returns `true` iff the field graph contains at least one triple
/// `?s rdf:type urn:ccog:GoalState` AND at least one triple
/// `?s rdf:type urn:ccog:CurrentState`. Both conditions must hold —
/// gap reduction without both endpoints is meaningless.
pub fn admit(field: &FieldContext) -> Result<bool> {
    let goal = NamedNode::new(GOAL_STATE_IRI)?;
    let current = NamedNode::new(CURRENT_STATE_IRI)?;
    if field.graph.instances_of(&goal)?.is_empty() {
        return Ok(false);
    }
    Ok(!field.graph.instances_of(&current)?.is_empty())
}

/// Materialize a [`GapReductionPlan`] from the current field state.
///
/// Walks both `GoalState` and `CurrentState` instance lists and pairs
/// them in sorted order. Any leftover unpaired goals contribute to
/// `residual_gaps` (saturating at 255). At most 8 moves are kept inline
/// in the returned `SmallVec`; further pairs spill to heap.
///
/// **Not on the `decide()` hot path.** Allocates an
/// `SmallVec<[ReductionMove; 8]>` and may issue graph queries.
pub fn materialize(field: &FieldContext) -> Result<GapReductionPlan> {
    let goal = NamedNode::new(GOAL_STATE_IRI)?;
    let current = NamedNode::new(CURRENT_STATE_IRI)?;
    let mut goals = field.graph.instances_of(&goal)?;
    let mut currents = field.graph.instances_of(&current)?;
    goals.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    currents.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    let mut moves: SmallVec<[ReductionMove; 8]> = SmallVec::new();
    let pair_count = goals.len().min(currents.len());
    for (g, c) in goals.iter().take(pair_count).zip(currents.iter()) {
        moves.push(ReductionMove {
            goal: GraphIri(g.clone()),
            current: GraphIri(c.clone()),
        });
    }
    let leftover = goals.len().saturating_sub(pair_count);
    let residual_gaps: u8 = if leftover > 255 { 255 } else { leftover as u8 };

    Ok(GapReductionPlan {
        root: GraphIri::from_iri(GPS_ROOT_IRI)?,
        moves,
        residual_gaps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_with(triples: &str) -> FieldContext {
        let mut f = FieldContext::new("gps");
        f.load_field_state(triples).expect("load");
        f
    }

    #[test]
    fn gps_admitted_when_goal_and_current_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/g1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:GoalState> .\n\
             <http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:CurrentState> .\n",
        );
        assert!(admit(&f)?);
        Ok(())
    }

    #[test]
    fn gps_denied_when_goal_missing() -> Result<()> {
        let f = field_with(
            "<http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:CurrentState> .\n",
        );
        assert!(!admit(&f)?);
        Ok(())
    }

    #[test]
    fn gps_materialize_pairs_sorted_subjects() -> Result<()> {
        let f = field_with(
            "<http://example.org/g2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:GoalState> .\n\
             <http://example.org/g1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:GoalState> .\n\
             <http://example.org/c1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:CurrentState> .\n",
        );
        let plan = materialize(&f)?;
        assert_eq!(plan.moves.len(), 1);
        assert_eq!(plan.moves[0].goal.as_str(), "http://example.org/g1");
        assert_eq!(plan.residual_gaps, 1);
        Ok(())
    }
}
