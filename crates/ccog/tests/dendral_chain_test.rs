use ccog::breeds::dendral::reconstruct_chain;
use ccog::graph::GraphIri;
use ccog::{process, FieldContext};

const NT_FIELD: &str = r#"
<http://example.org/claim/1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Claim> .
<http://example.org/evidence/police_report> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .
<http://example.org/evidence/police_report> <http://purl.org/dc/terms/type> <http://example.org/police_report_concept> .
<http://example.org/police_report_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/police_report_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "police report" .
<http://example.org/missing_concept> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .
<http://example.org/missing_concept> <http://www.w3.org/2004/02/skos/core#prefLabel> "missing" .
"#;

#[test]
fn dendral_walks_back_one_step_from_process_output() {
    let mut field = FieldContext::new("dendral");
    field.graph.load_ntriples(NT_FIELD).unwrap();
    let verdict = process("the police report is missing", &mut field).expect("process");
    let output_iri =
        GraphIri::from_iri(&format!("urn:ccog:output:{}", verdict.receipt.hash)).unwrap();
    let chain = reconstruct_chain(&output_iri, &field).expect("walk");
    assert_eq!(chain.root_entity.as_str(), output_iri.as_str());
    assert!(!chain.steps.is_empty(), "expected at least one PROV step");
    let expected_activity_hash = format!(
        "urn:ccog:id:{:08x}",
        ccog::utils::dense::fnv1a_64(verdict.receipt.activity_iri.as_str().as_bytes()) as u32
    );
    assert_eq!(chain.steps[0].activity.as_str(), expected_activity_hash);
}

#[test]
fn dendral_handles_unknown_entity() {
    let field = FieldContext::new("empty");
    let unknown = GraphIri::from_iri("urn:test:nonexistent").unwrap();
    let chain = reconstruct_chain(&unknown, &field).expect("walk");
    assert!(chain.steps.is_empty());
}
