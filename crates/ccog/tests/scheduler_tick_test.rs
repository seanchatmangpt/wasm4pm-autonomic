use ccog::hooks::missing_evidence_hook;
use ccog::{FieldContext, HookRegistry, Scheduler};

const NT_DOC1: &str = "<http://example.org/doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n";
const NT_DOC2: &str = "<http://example.org/doc2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n";

#[test]
fn scheduler_first_tick_fires_then_idempotent_then_refires() {
    let field = FieldContext::new("scheduler-test");
    let mut registry = HookRegistry::new();
    registry.register(missing_evidence_hook());
    let mut sched = Scheduler::new(registry);

    field.graph.load_ntriples(NT_DOC1).unwrap();
    let r1 = sched.tick(&field).unwrap();
    assert!(!r1.outcomes.is_empty());
    assert!(!r1.delta.is_empty());

    let r2 = sched.tick(&field).unwrap();
    // graph may have grown from r1 hook acts (which materialized a prov:value triple).
    // A SECOND tick on the same final state must produce no new delta.
    let r3 = sched.tick(&field).unwrap();
    assert!(r3.delta.is_empty(), "no further ΔO ⇒ empty delta");
    assert!(r3.outcomes.is_empty(), "no ΔO ⇒ no hooks fire");

    field.graph.load_ntriples(NT_DOC2).unwrap();
    let r4 = sched.tick(&field).unwrap();
    assert!(
        !r4.delta.is_empty(),
        "loading new triples ⇒ non-empty delta"
    );

    // suppress unused-warnings
    let _ = r2;
}
