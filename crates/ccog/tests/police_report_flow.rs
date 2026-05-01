use ccog::{process, FieldContext};

const TEST_FIELD_STATE: &str = r#"
<http://example.org/claim/1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Claim> .

<http://example.org/evidence/police_report> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .
<http://example.org/evidence/police_report> <http://purl.org/dc/terms/type> <http://example.org/police_report_concept> .

<http://example.org/police_report_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/police_report_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "police report" .

<http://example.org/missing_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/missing_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "missing" .
"#;

#[test]
fn test_police_report_mvp() {
    let mut field = FieldContext::new("claims");
    field
        .graph
        .load_ntriples(TEST_FIELD_STATE)
        .expect("Failed to load field state");

    let verdict = process("The police report is missing", &mut field).expect("process() failed");

    // Phrase bound to at least one public graph term
    assert!(
        !verdict.bound_terms.terms.is_empty(),
        "Expected bound terms from 'The police report is missing'"
    );

    // Both tokens should bind
    assert!(
        verdict.bound_terms.terms.len() >= 1,
        "Expected at least 1 bound term, got {}",
        verdict.bound_terms.terms.len()
    );

    // Evidence gap should be detected (no prov:value on police_report)
    let has_evidence_gap = verdict.evidence_gap.is_some();
    assert!(has_evidence_gap, "Expected evidence gap to be detected");

    // Transition should be blocked due to missing evidence
    assert!(
        !verdict.transition.admissible,
        "Expected transition to be blocked, got admissible={}",
        verdict.transition.admissible
    );

    // Candidate operation should be schema:AskAction
    assert_eq!(
        verdict.operation.kind_iri.as_str(),
        "https://schema.org/AskAction",
        "Expected schema:AskAction, got {}",
        verdict.operation.kind_iri.as_str()
    );

    // PROV receipt should be emitted
    let receipt_hash = &verdict.receipt.hash;
    assert_eq!(
        receipt_hash.len(),
        64,
        "BLAKE3 hex hash should be 64 chars, got {}",
        receipt_hash.len()
    );

    // No ccog: namespace should appear
    let ask_result = field
        .graph
        .ask("ASK { ?s ?p ?o . FILTER(STRSTARTS(STR(?s), 'http://ccog')) }")
        .expect("ASK query failed");
    assert!(!ask_result, "ccog: namespace should not appear in graph");
}
