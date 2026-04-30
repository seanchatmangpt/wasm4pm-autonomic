//! Bounded operational field with embedded RDF store.

use crate::graph::GraphStore;
use anyhow::Result;

/// FieldContext: bounded operational field U + embedded RDF store O*_U.
/// The field is the semantic closure and operational scope for cognitive passes.
#[derive(Debug)]
pub struct FieldContext {
    /// Name of the bounded operational field (e.g., "claims", "routing", "healthcare").
    pub name: String,

    /// Embedded RDF/SPARQL graph store containing public ontologies and field state.
    pub graph: GraphStore,
}

impl FieldContext {
    /// Create a new field context with a given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            graph: GraphStore::new(),
        }
    }

    /// Load field state from N-Triples format.
    pub fn load_field_state(&mut self, ntriples: &str) -> Result<()> {
        self.graph.load_ntriples(ntriples)
    }
}
