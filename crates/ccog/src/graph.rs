//! RDF graph store wrapper with public-ontology-only SPARQL execution.

use anyhow::{anyhow, Result};
use oxigraph::io::RdfFormat;
use oxigraph::model::*;
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use std::fmt;

/// Public RDF ontology prefixes for all SPARQL queries.
/// Central prefix management ensures consistency across all cognitive passes.
pub const PREFIXES: &str = r#"
PREFIX rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX xsd:     <http://www.w3.org/2001/XMLSchema#>
PREFIX skos:    <http://www.w3.org/2004/02/skos/core#>
PREFIX schema:  <https://schema.org/>
PREFIX prov:    <http://www.w3.org/ns/prov#>
PREFIX dcterms: <http://purl.org/dc/terms/>
PREFIX sh:      <http://www.w3.org/ns/shacl#>
PREFIX odrl:    <http://www.w3.org/ns/odrl/2/>
"#;

/// Wrapper around Oxigraph store with public-ontology-only SPARQL execution.
pub struct GraphStore {
    store: Store,
}

impl fmt::Debug for GraphStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GraphStore").finish()
    }
}

impl GraphStore {
    /// Create a new in-memory graph store.
    pub fn new() -> Self {
        Self {
            store: Store::new().expect("Failed to create Oxigraph store"),
        }
    }

    /// Load RDF triples from N-Triples format.
    pub fn load_ntriples(&self, ntriples: &str) -> Result<()> {
        self.store
            .load_from_reader(RdfFormat::NTriples, ntriples.as_bytes())
            .map_err(|e| anyhow!("Failed to load N-Triples: {}", e))?;
        Ok(())
    }

    /// Execute a SPARQL SELECT query with automatic prefix prepending.
    /// Returns rows of (variable_name_without_question_mark, NamedNode) — literals
    /// and blank nodes are filtered. Use [`select_terms`] for full term access.
    pub fn select(&self, sparql: &str) -> Result<Vec<Vec<(String, NamedNode)>>> {
        let query = format!("{}{}", PREFIXES, sparql);
        let results = SparqlEvaluator::new()
            .parse_query(&query)
            .map_err(|e| anyhow!("SPARQL parse error: {}", e))?
            .on_store(&self.store)
            .execute()
            .map_err(|e| anyhow!("SPARQL SELECT failed: {}", e))?;

        if let QueryResults::Solutions(bindings) = results {
            let mut rows = Vec::new();
            for row_result in bindings {
                let row =
                    row_result.map_err(|e| anyhow!("Failed to read query result row: {}", e))?;
                let mut r = Vec::new();
                for (var, term) in row.iter() {
                    if let Term::NamedNode(node) = term {
                        let name = var.as_str().trim_start_matches('?').to_string();
                        r.push((name, node.clone()));
                    }
                }
                rows.push(r);
            }
            Ok(rows)
        } else {
            Err(anyhow!("SPARQL query did not return SELECT results"))
        }
    }

    /// Execute a SPARQL ASK query with automatic prefix prepending.
    pub fn ask(&self, sparql: &str) -> Result<bool> {
        let query = format!("{}{}", PREFIXES, sparql);
        let results = SparqlEvaluator::new()
            .parse_query(&query)
            .map_err(|e| anyhow!("SPARQL parse error: {}", e))?
            .on_store(&self.store)
            .execute()
            .map_err(|e| anyhow!("SPARQL ASK failed: {}", e))?;

        if let QueryResults::Boolean(b) = results {
            Ok(b)
        } else {
            Err(anyhow!("SPARQL query did not return ASK result"))
        }
    }

    /// Execute a SPARQL CONSTRUCT query with automatic prefix prepending.
    pub fn construct(&self, sparql: &str) -> Result<Vec<Triple>> {
        let query = format!("{}{}", PREFIXES, sparql);
        let results = SparqlEvaluator::new()
            .parse_query(&query)
            .map_err(|e| anyhow!("SPARQL parse error: {}", e))?
            .on_store(&self.store)
            .execute()
            .map_err(|e| anyhow!("SPARQL CONSTRUCT failed: {}", e))?;

        if let QueryResults::Graph(triples) = results {
            let mut t = Vec::new();
            for triple_result in triples {
                let triple =
                    triple_result.map_err(|e| anyhow!("Failed to read CONSTRUCT result: {}", e))?;
                t.push(triple);
            }
            Ok(t)
        } else {
            Err(anyhow!("SPARQL query did not return CONSTRUCT results"))
        }
    }

    /// Insert RDF triples from N-Triples format as delta assertion.
    ///
    /// Semantically distinct from [`load_ntriples`] which is used for corpus loading.
    /// This method is specifically for asserting new triples into the graph.
    pub fn insert_ntriples(&self, ntriples: &str) -> Result<()> {
        self.store
            .load_from_reader(RdfFormat::NTriples, ntriples.as_bytes())
            .map_err(|e| anyhow!("Failed to insert N-Triples: {}", e))?;
        Ok(())
    }

    /// Insert a batch of RDF triples directly into the default graph.
    ///
    /// Uses Oxigraph's native `Store::insert` (no SPARQL or N-Triples round-trip).
    /// Returns the count of triples submitted (not deduplicated insert count).
    pub fn insert_triples(&self, triples: &[Triple]) -> Result<usize> {
        let count = triples.len();
        if count == 0 {
            return Ok(0);
        }
        for triple in triples {
            let quad = Quad::new(
                triple.subject.clone(),
                triple.predicate.clone(),
                triple.object.clone(),
                GraphName::DefaultGraph,
            );
            self.store
                .insert(&quad)
                .map_err(|e| anyhow!("Failed to insert quad: {}", e))?;
        }
        Ok(count)
    }

    /// Insert a batch of optional RDF triples, filtering out `None` slots.
    ///
    /// Ignores `None` entries in the input slice and delegates to [`insert_triples`]
    /// with only the concrete (Some) triples. Returns the count of triples inserted.
    pub fn insert_optional_triples(&self, triples: &[Option<Triple>]) -> Result<usize> {
        let concrete: Vec<Triple> = triples.iter().filter_map(|t| t.clone()).collect();
        self.insert_triples(&concrete)
    }

    /// Return every triple in the store via direct `quads_for_pattern` enumeration.
    ///
    /// Used by the runtime ΔO detector to capture full graph snapshots.
    /// Bypasses SPARQL entirely — direct iteration over the store.
    pub fn all_triples(&self) -> Result<Vec<Triple>> {
        let mut out = Vec::new();
        for quad_result in self.store.quads_for_pattern(None, None, None, None) {
            let quad = quad_result.map_err(|e| anyhow!("Failed to enumerate quads: {}", e))?;
            out.push(Triple::new(quad.subject, quad.predicate, quad.object));
        }
        Ok(out)
    }

    /// Find all named-node objects of `(subject, predicate, ?)`.
    ///
    /// Direct triple-pattern lookup — no SPARQL parsing. Filters non-IRI objects.
    pub fn objects_of(&self, subject: &NamedNode, predicate: &NamedNode) -> Result<Vec<NamedNode>> {
        let s_ref: NamedOrBlankNodeRef = subject.into();
        let p_ref: NamedNodeRef = predicate.into();
        let mut out = Vec::new();
        for quad_result in
            self.store
                .quads_for_pattern(Some(s_ref), Some(p_ref), None, None)
        {
            let quad = quad_result.map_err(|e| anyhow!("quads_for_pattern: {}", e))?;
            if let Term::NamedNode(n) = quad.object {
                out.push(n);
            }
        }
        Ok(out)
    }

    /// Find all named-node subjects of `(?, predicate, object)`.
    ///
    /// Direct triple-pattern lookup — no SPARQL parsing. Filters non-IRI subjects.
    pub fn subjects_with(
        &self,
        predicate: &NamedNode,
        object: &NamedNode,
    ) -> Result<Vec<NamedNode>> {
        let p_ref: NamedNodeRef = predicate.into();
        let o_ref: TermRef = object.into();
        let mut out = Vec::new();
        for quad_result in
            self.store
                .quads_for_pattern(None, Some(p_ref), Some(o_ref), None)
        {
            let quad = quad_result.map_err(|e| anyhow!("quads_for_pattern: {}", e))?;
            if let NamedOrBlankNode::NamedNode(n) = quad.subject {
                out.push(n);
            }
        }
        Ok(out)
    }

    /// True if the graph contains any quad matching `(subject?, predicate?, object?)`.
    ///
    /// Any `None` component is a wildcard. Short-circuits on the first match.
    pub fn pattern_exists(
        &self,
        subject: Option<&NamedNode>,
        predicate: Option<&NamedNode>,
        object: Option<&Term>,
    ) -> Result<bool> {
        let s_ref = subject.map(|s| {
            let r: NamedOrBlankNodeRef = s.into();
            r
        });
        let p_ref = predicate.map(|p| {
            let r: NamedNodeRef = p.into();
            r
        });
        let o_ref = object.map(|o| {
            let r: TermRef = o.into();
            r
        });
        let mut iter = self.store.quads_for_pattern(s_ref, p_ref, o_ref, None);
        match iter.next() {
            Some(Ok(_)) => Ok(true),
            Some(Err(e)) => Err(anyhow!("quads_for_pattern: {}", e)),
            None => Ok(false),
        }
    }

    /// True if the graph contains any quad matching `(subject, predicate, ?)`.
    ///
    /// Short-circuits on the first match — no full enumeration.
    pub fn has_value_for(&self, subject: &NamedNode, predicate: &NamedNode) -> Result<bool> {
        let s_ref: NamedOrBlankNodeRef = subject.into();
        let p_ref: NamedNodeRef = predicate.into();
        let mut iter = self
            .store
            .quads_for_pattern(Some(s_ref), Some(p_ref), None, None);
        match iter.next() {
            Some(Ok(_)) => Ok(true),
            Some(Err(e)) => Err(anyhow!("quads_for_pattern: {}", e)),
            None => Ok(false),
        }
    }

    /// Find all named-node instances of `class` (subjects with `rdf:type class`).
    ///
    /// Direct triple-pattern lookup — no SPARQL parsing.
    pub fn instances_of(&self, class: &NamedNode) -> Result<Vec<NamedNode>> {
        let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
        self.subjects_with(&rdf_type, class)
    }

    /// Iterate triples matching `(?, predicate, ?)` and yield `(subject, object)` pairs.
    ///
    /// Used by ELIZA to build a label index in one pass without SPARQL.
    /// Subjects must be NamedNodes; object terms are returned verbatim (literals OK).
    pub fn pairs_with_predicate(
        &self,
        predicate: &NamedNode,
    ) -> Result<Vec<(NamedNode, Term)>> {
        let p_ref: NamedNodeRef = predicate.into();
        let mut out = Vec::new();
        for quad_result in self.store.quads_for_pattern(None, Some(p_ref), None, None) {
            let quad = quad_result.map_err(|e| anyhow!("quads_for_pattern: {}", e))?;
            if let NamedOrBlankNode::NamedNode(s) = quad.subject {
                out.push((s, quad.object));
            }
        }
        Ok(out)
    }

    /// Execute a SPARQL SELECT query and return all term types, not just named nodes.
    ///
    /// Like [`select`], but preserves literals, blank nodes, and other RDF term types
    /// instead of filtering to only named nodes. Strips `?` from variable names.
    pub fn select_terms(&self, sparql: &str) -> Result<Vec<Vec<(String, Term)>>> {
        let query = format!("{}{}", PREFIXES, sparql);
        let results = SparqlEvaluator::new()
            .parse_query(&query)
            .map_err(|e| anyhow!("SPARQL parse error: {}", e))?
            .on_store(&self.store)
            .execute()
            .map_err(|e| anyhow!("SPARQL SELECT failed: {}", e))?;

        if let QueryResults::Solutions(bindings) = results {
            let mut rows = Vec::new();
            for row_result in bindings {
                let row =
                    row_result.map_err(|e| anyhow!("Failed to read query result row: {}", e))?;
                let mut r = Vec::new();
                for (var, term) in row.iter() {
                    let name = var.as_str().trim_start_matches('?').to_string();
                    r.push((name, term.clone()));
                }
                rows.push(r);
            }
            Ok(rows)
        } else {
            Err(anyhow!("SPARQL query did not return SELECT results"))
        }
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper type for Named Node IRIs to use throughout ccog.
/// Use full IRIs internally — never prefix strings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GraphIri(pub NamedNode);

impl GraphIri {
    /// Create a GraphIri from a full IRI string.
    pub fn from_iri(iri: &str) -> Result<Self> {
        Ok(GraphIri(
            NamedNode::new(iri).map_err(|e| anyhow!("Invalid IRI '{}': {}", iri, e))?,
        ))
    }

    /// Get the IRI as a string reference.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NamedNode> for GraphIri {
    fn from(nn: NamedNode) -> Self {
        GraphIri(nn)
    }
}

impl From<GraphIri> for NamedNode {
    fn from(gi: GraphIri) -> Self {
        gi.0
    }
}
