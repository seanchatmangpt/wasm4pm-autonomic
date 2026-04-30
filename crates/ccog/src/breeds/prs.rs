//! PRS breed (Phase 9): Procedural Reasoning System / BDI commit.
//!
//! PRS is admitted iff the field graph contains at least one typed instance
//! of each of `urn:ccog:Belief`, `urn:ccog:Desire`, and `urn:ccog:Intention`.
//! Admission is a pure triple-pattern probe; no SPARQL parsing.
//!
//! Materialization is **not** on the `decide()` path. It walks each instance
//! list, picks the lexicographically smallest IRI per role, and returns a
//! single [`IntentionCommit`] (with `committed = false` by default — the
//! caller is responsible for pinning the intention via a separate act).

use anyhow::Result;
use oxigraph::model::NamedNode;

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::IntentionCommit;

const BELIEF_IRI: &str = "urn:ccog:Belief";
const DESIRE_IRI: &str = "urn:ccog:Desire";
const INTENTION_IRI: &str = "urn:ccog:Intention";

/// Probe whether PRS is admissible against `field`.
///
/// Returns `true` iff all three of `urn:ccog:Belief`, `urn:ccog:Desire`,
/// and `urn:ccog:Intention` have at least one typed subject in the graph.
pub fn admit(field: &FieldContext) -> Result<bool> {
    let belief = NamedNode::new(BELIEF_IRI)?;
    let desire = NamedNode::new(DESIRE_IRI)?;
    let intention = NamedNode::new(INTENTION_IRI)?;
    if field.graph.instances_of(&belief)?.is_empty() {
        return Ok(false);
    }
    if field.graph.instances_of(&desire)?.is_empty() {
        return Ok(false);
    }
    Ok(!field.graph.instances_of(&intention)?.is_empty())
}

/// Materialize a single [`IntentionCommit`] from the lexicographically
/// smallest belief / desire / intention triple.
///
/// **Not on the `decide()` hot path.** Returns `None` if any of the three
/// roles is unpopulated. `committed` defaults to `false`; pinning happens
/// outside this materialization (e.g., by writing back via Construct8).
pub fn materialize(field: &FieldContext) -> Result<Option<IntentionCommit>> {
    let belief = NamedNode::new(BELIEF_IRI)?;
    let desire = NamedNode::new(DESIRE_IRI)?;
    let intention = NamedNode::new(INTENTION_IRI)?;
    let mut beliefs = field.graph.instances_of(&belief)?;
    let mut desires = field.graph.instances_of(&desire)?;
    let mut intentions = field.graph.instances_of(&intention)?;
    if beliefs.is_empty() || desires.is_empty() || intentions.is_empty() {
        return Ok(None);
    }
    beliefs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    desires.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    intentions.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(Some(IntentionCommit {
        belief: GraphIri(beliefs.into_iter().next().unwrap()),
        desire: GraphIri(desires.into_iter().next().unwrap()),
        intention: GraphIri(intentions.into_iter().next().unwrap()),
        committed: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_with(triples: &str) -> FieldContext {
        let mut f = FieldContext::new("prs");
        f.load_field_state(triples).expect("load");
        f
    }

    #[test]
    fn prs_admitted_with_full_bdi_triple() -> Result<()> {
        let f = field_with(
            "<http://example.org/b1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Belief> .\n\
             <http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Desire> .\n\
             <http://example.org/i1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Intention> .\n",
        );
        assert!(admit(&f)?);
        let commit = materialize(&f)?.expect("must yield a commit");
        assert_eq!(commit.belief.as_str(), "http://example.org/b1");
        assert_eq!(commit.desire.as_str(), "http://example.org/d1");
        assert_eq!(commit.intention.as_str(), "http://example.org/i1");
        assert!(!commit.committed);
        Ok(())
    }

    #[test]
    fn prs_denied_missing_intention() -> Result<()> {
        let f = field_with(
            "<http://example.org/b1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Belief> .\n\
             <http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Desire> .\n",
        );
        assert!(!admit(&f)?);
        Ok(())
    }
}
