use ccog::breeds::prolog::prove_relation;
use ccog::graph::GraphIri;
use ccog::FieldContext;

const NT_CHAIN: &str = r#"
<http://example.org/a> <http://www.w3.org/2004/02/skos/core#broader> <http://example.org/b> .
<http://example.org/b> <http://www.w3.org/2004/02/skos/core#broader> <http://example.org/c> .
<http://example.org/x> <http://www.w3.org/2004/02/skos/core#broader> <http://example.org/y> .
"#;

#[test]
fn prolog_proves_transitive_skos_broader() {
    let field = FieldContext::new("relations");
    field.graph.load_ntriples(NT_CHAIN).unwrap();
    let a = GraphIri::from_iri("http://example.org/a").unwrap();
    let c = GraphIri::from_iri("http://example.org/c").unwrap();
    let pred = GraphIri::from_iri("http://www.w3.org/2004/02/skos/core#broader").unwrap();
    let proof = prove_relation(&a, &pred, &c, &field)
        .expect("prove")
        .expect("Some");
    let path: Vec<&str> = proof.path.iter().map(|i| i.as_str()).collect();
    assert_eq!(*path.first().unwrap(), "http://example.org/a");
    assert_eq!(*path.last().unwrap(), "http://example.org/c");
    assert!(path.contains(&"http://example.org/b"));
}

#[test]
fn prolog_returns_none_when_no_path() {
    let field = FieldContext::new("relations-none");
    field.graph.load_ntriples(NT_CHAIN).unwrap();
    let a = GraphIri::from_iri("http://example.org/a").unwrap();
    let y = GraphIri::from_iri("http://example.org/y").unwrap();
    let pred = GraphIri::from_iri("http://www.w3.org/2004/02/skos/core#broader").unwrap();
    assert!(prove_relation(&a, &pred, &y, &field)
        .expect("prove")
        .is_none());
}
