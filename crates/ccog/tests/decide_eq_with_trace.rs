//! Phase 7 equivalence invariant: `decide_with_trace` must produce the
//! exact same `BarkDecision` as `decide` for any snapshot. The trace is
//! diagnostic — adding it must never change the canonical decision.

use ccog::bark_artifact::decide;
use ccog::trace::decide_with_trace;
use ccog::{CompiledFieldSnapshot, FieldContext};
use proptest::prelude::*;

#[test]
fn decide_eq_decide_with_trace_on_empty_field() {
    let field = FieldContext::new("test");
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let canonical = decide(&snap);
    let (with_trace, _trace) = decide_with_trace(&snap);
    assert_eq!(canonical, with_trace);
}

#[test]
fn decide_eq_decide_with_trace_on_loaded_field() {
    let mut field = FieldContext::new("test");
    field
        .load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )
        .unwrap();
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let canonical = decide(&snap);
    let (with_trace, _trace) = decide_with_trace(&snap);
    assert_eq!(canonical, with_trace);
}

// Build a synthetic field containing a random subset of the canonical
// predicate-bearing triples. Each bit in `pred_bits` toggles one fixture.
fn field_with_predicates(pred_bits: u8) -> FieldContext {
    let mut field = FieldContext::new("proptest");
    let mut nt = String::new();
    if pred_bits & 0b0000_0001 != 0 {
        nt.push_str("<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n");
    }
    if pred_bits & 0b0000_0010 != 0 {
        nt.push_str("<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"v\" .\n");
    }
    if pred_bits & 0b0000_0100 != 0 {
        nt.push_str("<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n");
    }
    if pred_bits & 0b0000_1000 != 0 {
        nt.push_str("<http://example.org/c2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n");
    }
    if pred_bits & 0b0001_0000 != 0 {
        nt.push_str("<http://example.org/c3> <http://purl.org/dc/terms/type> <http://example.org/Foo> .\n");
    }
    if pred_bits & 0b0010_0000 != 0 {
        nt.push_str("<http://example.org/c4> <http://www.w3.org/2000/01/rdf-schema#label> \"y\" .\n");
    }
    if pred_bits & 0b0100_0000 != 0 {
        nt.push_str("<http://example.org/d2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n");
        nt.push_str("<http://example.org/d2> <http://www.w3.org/ns/prov#value> \"v2\" .\n");
    }
    if !nt.is_empty() {
        field.load_field_state(&nt).unwrap();
    }
    field
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn decide_eq_decide_with_trace_proptest(pred_bits in any::<u8>()) {
        let field = field_with_predicates(pred_bits);
        let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
        let canonical = decide(&snap);
        let (with_trace, _trace) = decide_with_trace(&snap);
        prop_assert_eq!(canonical, with_trace);
    }
}
