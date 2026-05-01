//! SHRDLU breed: object-affordance assessment via direct triple-pattern walks.

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::AffordanceVerdict;
use anyhow::Result;
use oxigraph::model::NamedNode;

/// SHRDLU: Returns the set of admissible actions for an object using
/// `schema:potentialAction` and reverse `schema:object` edges.
/// Direct triple-pattern walk — no SPARQL parsing.
pub fn check_affordance(object_iri: &GraphIri, field: &FieldContext) -> Result<AffordanceVerdict> {
    let schema_object = NamedNode::new("https://schema.org/object")?;
    let schema_potential_action = NamedNode::new("https://schema.org/potentialAction")?;
    let obj_node: NamedNode = object_iri.0.clone();

    let mut actions: Vec<NamedNode> = Vec::new();
    actions.extend(field.graph.subjects_with(&schema_object, &obj_node)?);
    actions.extend(
        field
            .graph
            .objects_of(&obj_node, &schema_potential_action)?,
    );

    let mut wrapped: Vec<GraphIri> = actions.into_iter().map(GraphIri).collect();
    wrapped.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    wrapped.dedup_by(|a, b| a.as_str() == b.as_str());
    Ok(AffordanceVerdict {
        object: object_iri.clone(),
        actions: wrapped,
    })
}
