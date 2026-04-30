//! Integration test for PROV materialization.
//!
//! Tests that the ccog process correctly materializes PROV-O triples
//! when processing phrases through the field context.

use ccog::{FieldContext, process};

/// Field state fixture with PROV-compatible ontology.
///
/// Contains a claim with associated evidence documentation and SKOS concepts.
const NT_PROV_FIELD: &str = r#"
<http://example.org/claim/42> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Claim> .
<http://example.org/evidence/photo> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .
<http://example.org/evidence/photo> <http://purl.org/dc/terms/type> <http://example.org/photo_concept> .
<http://example.org/photo_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/photo_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "photo evidence" .
"#;

/// Test that PROV delta materializes prov:Activity triples.
///
/// Verifies that after processing a phrase, the graph contains at least one
/// prov:Activity instance that records the cognitive activity. Also ensures
/// no http://ccog namespace IRIs are materialized (only urn: scheme allowed).
#[test]
fn prov_delta_materializes_activity_triples() {
    let mut field = FieldContext::new("claims-prov");
    field.load_field_state(NT_PROV_FIELD).expect("Failed to load field state");

    let verdict = process("The photo evidence is missing", &mut field).expect("process() failed");

    // Assert: graph contains prov:Activity
    let has_activity = field.graph.ask("ASK { ?a a prov:Activity }").expect("ASK failed");
    assert!(
        has_activity,
        "Graph must contain prov:Activity after process()"
    );

    // Assert: no http://ccog: namespace (URNs are OK)
    let has_ccog_http = field.graph.ask(
        "ASK { ?s ?p ?o . FILTER(STRSTARTS(STR(?s), 'http://ccog')) }"
    ).expect("ASK namespace check failed");
    assert!(
        !has_ccog_http,
        "No http://ccog: IRIs allowed; only urn: scheme"
    );

    // Assert: receipt hash is valid BLAKE3 (64 hex chars)
    assert_eq!(
        verdict.receipt.hash.len(),
        64,
        "BLAKE3 hash must be 64 chars"
    );
}

/// Test that PROV delta materialization is idempotent.
///
/// Verifies that calling process() twice with identical inputs produces
/// the same graph state (via Oxigraph set semantics). This is critical
/// for deterministic and reproducible PROV-O semantics.
#[test]
fn prov_delta_is_idempotent() {
    let mut field = FieldContext::new("claims-prov-idem");
    field.load_field_state(NT_PROV_FIELD).expect("Failed to load");

    process("The photo evidence is missing", &mut field).expect("First process() failed");
    process("The photo evidence is missing", &mut field).expect("Second process() failed");

    // Count activities — should be exactly 1 (Oxigraph set semantics)
    let rows = field
        .graph
        .select("SELECT ?a WHERE { ?a a prov:Activity }")
        .expect("SELECT failed");
    assert_eq!(
        rows.len(),
        1,
        "emit_prov_delta() must be idempotent; expected exactly 1 prov:Activity"
    );
}
