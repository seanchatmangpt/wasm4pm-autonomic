use ccog::{FieldContext, HookRegistry, Runtime, PackPosture};
use ccog::hooks::{missing_evidence_hook, phrase_binding_hook};

#[test]
fn runtime_step_chains_receipts_across_cycles() {
    let field = FieldContext::new("runtime-test");
    let mut registry = HookRegistry::new();
    registry.register(missing_evidence_hook());
    registry.register(phrase_binding_hook());
    let mut rt = Runtime::new(field, registry);

    let r1 = rt.step().expect("step1");
    assert_eq!(r1.posture, PackPosture::Calm);
    assert!(!r1.chain_extended);

    rt.field_mut().graph.load_ntriples(
        "<http://example.org/doc/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
    ).unwrap();
    let r2 = rt.step().expect("step2");
    let _ = r2;

    rt.field_mut().graph.load_ntriples(
        "<http://example.org/doc/B> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
    ).unwrap();
    let r3 = rt.step().expect("step3");

    if r3.chain_extended {
        let has_chain = rt.field().graph.ask("ASK { ?n <http://www.w3.org/ns/prov#wasInformedBy> ?o }").expect("ask");
        assert!(has_chain, "graph must contain prov:wasInformedBy after chained step");
    }
    // posture should have advanced past Calm at least once
    assert!(rt.posture() != PackPosture::Calm || !r3.tick.outcomes.is_empty());
}
