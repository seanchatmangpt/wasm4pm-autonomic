//! MYCIN breed: evidence-gap detection via direct triple-pattern walks.

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::{BoundTerms, EvidenceGap};
use anyhow::Result;
use oxigraph::model::NamedNode;

/// MYCIN: Evidence-gap detection.
/// For each bound concept, find entities typed as that concept which lack `prov:value`.
/// Direct triple-pattern walk — no SPARQL parsing.
pub fn find_missing_evidence(
    bound_terms: &BoundTerms,
    field: &FieldContext,
) -> Result<Option<EvidenceGap>> {
    if bound_terms.terms.is_empty() {
        return Ok(None);
    }

    let dcterms_type = NamedNode::new("http://purl.org/dc/terms/type")?;
    let prov_value = NamedNode::new("http://www.w3.org/ns/prov#value")?;

    let mut missing = Vec::new();
    for concept in &bound_terms.terms {
        let concept_node: NamedNode = concept.0.clone();
        let entities = field.graph.subjects_with(&dcterms_type, &concept_node)?;
        for entity in entities {
            if !field.graph.has_value_for(&entity, &prov_value)? {
                missing.push(GraphIri(entity));
            }
        }
    }

    if missing.is_empty() {
        return Ok(None);
    }

    missing.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    missing.dedup_by(|a, b| a.as_str() == b.as_str());
    Ok(Some(EvidenceGap { missing }))
}
