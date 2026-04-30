//! DENDRAL breed: backward PROV walk from an entity through generating activities.

use anyhow::Result;
use oxigraph::model::NamedNode;
use std::collections::{BTreeMap, HashSet};
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::{ProvenanceChain, ProvenanceStep};

const MAX_DEPTH: usize = 8;

/// Walk backward from `entity_iri` via prov:wasGeneratedBy → prov:used edges.
/// Depth-capped at 8. Direct triple-pattern walk — no SPARQL parsing.
pub fn reconstruct_chain(entity_iri: &GraphIri, field: &FieldContext) -> Result<ProvenanceChain> {
    let prov_was_generated_by = NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")?;
    let prov_used = NamedNode::new("http://www.w3.org/ns/prov#used")?;

    let mut steps: Vec<ProvenanceStep> = Vec::new();
    let mut frontier: Vec<NamedNode> = vec![entity_iri.0.clone()];
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(entity_iri.as_str().to_string());

    while !frontier.is_empty() && steps.len() < MAX_DEPTH {
        let mut next_frontier: Vec<NamedNode> = Vec::new();
        for current in frontier.drain(..) {
            let mut activities = field.graph.objects_of(&current, &prov_was_generated_by)?;
            activities.sort_by(|a, b| a.as_str().cmp(b.as_str()));

            let mut by_activity: BTreeMap<String, (NamedNode, Vec<NamedNode>)> = BTreeMap::new();
            for act in activities {
                let mut inputs = field.graph.objects_of(&act, &prov_used)?;
                inputs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                inputs.dedup_by(|a, b| a.as_str() == b.as_str());
                by_activity.insert(act.as_str().to_string(), (act, inputs));
            }
            for (_, (activity, inputs)) in by_activity {
                for inp in &inputs {
                    let s = inp.as_str().to_string();
                    if !visited.contains(&s) {
                        visited.insert(s);
                        next_frontier.push(inp.clone());
                    }
                }
                let inputs_wrapped: Vec<GraphIri> = inputs.into_iter().map(GraphIri).collect();
                steps.push(ProvenanceStep {
                    activity: GraphIri(activity),
                    inputs: inputs_wrapped,
                });
                if steps.len() >= MAX_DEPTH {
                    break;
                }
            }
            if steps.len() >= MAX_DEPTH {
                break;
            }
        }
        frontier = next_frontier;
    }
    Ok(ProvenanceChain {
        root_entity: entity_iri.clone(),
        steps,
    })
}
