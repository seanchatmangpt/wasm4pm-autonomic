use ccog::breeds::shrdlu::check_affordance;
use ccog::graph::GraphIri;
use ccog::FieldContext;

const NT_FIELD: &str = r#"
<http://example.org/obj1> <https://schema.org/potentialAction> <http://example.org/act1> .
<http://example.org/obj1> <https://schema.org/potentialAction> <http://example.org/act2> .
<http://example.org/act3> <https://schema.org/object> <http://example.org/obj1> .
<http://example.org/obj2> <https://schema.org/potentialAction> <http://example.org/actX> .
"#;

#[test]
fn shrdlu_returns_admissible_actions_for_object() {
    let mut field = FieldContext::new("affordance");
    field.graph.load_ntriples(NT_FIELD).unwrap();
    let obj = GraphIri::from_iri("http://example.org/obj1").unwrap();
    let v = check_affordance(&obj, &field).expect("check_affordance");
    let iris: Vec<&str> = v.actions.iter().map(|i| i.as_str()).collect();
    assert_eq!(v.object.as_str(), "http://example.org/obj1");
    assert_eq!(v.actions.len(), 3);
    assert!(iris.contains(&"http://example.org/act1"));
    assert!(iris.contains(&"http://example.org/act2"));
    assert!(iris.contains(&"http://example.org/act3"));
    assert!(!iris.contains(&"http://example.org/actX"));
}

#[test]
fn shrdlu_empty_actions_for_unknown_object() {
    let field = FieldContext::new("empty");
    let obj = GraphIri::from_iri("http://example.org/nope").unwrap();
    let v = check_affordance(&obj, &field).expect("check_affordance");
    assert!(v.actions.is_empty());
}
