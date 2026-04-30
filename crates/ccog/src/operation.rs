//! Public-ontology operation representation.

use crate::graph::GraphIri;

/// Candidate operation representation using public ontology terms only.
/// No custom ontology classes or predicates — schema:Action, schema:AskAction, SKOS concepts only.
#[derive(Clone, Debug)]
pub struct Operation {
    /// Full IRI of the operation type (e.g., "https://schema.org/AskAction").
    pub kind_iri: GraphIri,

    /// Human-readable label from SKOS or schema vocabulary (e.g., "Request missing evidence").
    pub label: Option<String>,

    /// Object being acted upon (e.g., the entity the operation targets).
    pub object: Option<GraphIri>,
}

impl Operation {
    /// Create a new operation with a full IRI and optional label.
    pub fn new(kind_iri: GraphIri, label: Option<String>) -> Self {
        Self {
            kind_iri,
            label,
            object: None,
        }
    }

    /// Set the object this operation acts upon.
    pub fn with_object(mut self, object: GraphIri) -> Self {
        self.object = Some(object);
        self
    }
}
