//! DENDRAL breed: backward PROV walk from an entity through generating activities.

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::utils::dense::{fnv1a_64, PackedKeyTable};
use crate::verdict::{ProvenanceChain, ProvenanceStep};
use anyhow::Result;
use oxigraph::model::NamedNode;
use std::collections::HashSet;

const MAX_DEPTH: usize = 8;

/// Walk backward from `entity_iri` via prov:wasGeneratedBy → prov:used edges.
/// Depth-capped at 8. Direct triple-pattern walk — no SPARQL parsing.
pub fn reconstruct_chain(entity_iri: &GraphIri, field: &FieldContext) -> Result<ProvenanceChain> {
    let prov_was_generated_by = NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")?;
    let prov_used = NamedNode::new("http://www.w3.org/ns/prov#used")?;

    // Numeric equivalents (Phase 6/v0.8)
    let h_gen = format!(
        "urn:ccog:p:{:04x}",
        crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#wasGeneratedBy".as_bytes()) as u16
    );
    let p_gen_num = NamedNode::new(&h_gen)?;
    let h_used = format!(
        "urn:ccog:p:{:04x}",
        crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#used".as_bytes()) as u16
    );
    let p_used_num = NamedNode::new(&h_used)?;

    let mut steps: Vec<ProvenanceStep> = Vec::new();
    let mut frontier: Vec<NamedNode> = vec![entity_iri.0.clone()];

    // Also add the hashed URN equivalent to the frontier
    let h_ent = format!(
        "urn:ccog:id:{:08x}",
        crate::utils::dense::fnv1a_64(entity_iri.as_str().as_bytes()) as u32
    );
    if let Ok(bn) = NamedNode::new(&h_ent) {
        frontier.push(bn);
    }

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(entity_iri.as_str().to_string());

    while !frontier.is_empty() && steps.len() < MAX_DEPTH {
        let mut next_frontier: Vec<NamedNode> = Vec::new();
        for current in frontier.drain(..) {
            let mut activities = field.graph.objects_of(&current, &prov_was_generated_by)?;
            activities.extend(field.graph.objects_of(&current, &p_gen_num)?);
            activities.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            activities.dedup();

            let mut by_activity: PackedKeyTable<String, (NamedNode, Vec<NamedNode>)> =
                PackedKeyTable::new();
            for act in activities {
                let mut inputs = field.graph.objects_of(&act, &prov_used)?;
                inputs.extend(field.graph.objects_of(&act, &p_used_num)?);
                inputs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                inputs.dedup_by(|a, b| a.as_str() == b.as_str());
                let key = act.as_str().to_string();
                by_activity.insert(fnv1a_64(key.as_bytes()), key, (act, inputs));
            }
            for (_, _, (activity, inputs)) in by_activity.iter() {
                for inp in inputs {
                    let s = inp.as_str().to_string();
                    if !visited.contains(&s) {
                        visited.insert(s);
                        next_frontier.push(inp.clone());
                    }
                }
                let inputs_wrapped: Vec<GraphIri> =
                    inputs.iter().map(|n| GraphIri(n.clone())).collect();
                steps.push(ProvenanceStep {
                    activity: GraphIri(activity.clone()),
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
