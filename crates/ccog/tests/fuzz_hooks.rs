use ccog::hooks::{
    missing_evidence_hook, phrase_binding_hook, receipt_hook, transition_admissibility_hook,
};
use ccog::{FieldContext, HookRegistry};
use proptest::prelude::*;

fn arb_hook_idx() -> impl Strategy<Value = usize> {
    0usize..4
}

fn arb_ntriples() -> impl Strategy<Value = String> {
    "[a-z]{3,6}".prop_map(|s| format!(
        "<http://example.org/{0}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        s
    ))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn fuzz_hooks_never_panic(
        hook_picks in prop::collection::vec(arb_hook_idx(), 0..6),
        triples in prop::collection::vec(arb_ntriples(), 0..6),
    ) {
        let field = FieldContext::new("fuzz");
        for nt in &triples { let _ = field.graph.load_ntriples(nt); }
        let mut registry = HookRegistry::new();
        for idx in &hook_picks {
            let hook = match *idx {
                0 => missing_evidence_hook(),
                1 => phrase_binding_hook(),
                2 => transition_admissibility_hook(),
                _ => receipt_hook(),
            };
            registry.register(hook);
        }
        let outcomes = registry.fire_matching(&field).unwrap_or_default();
        prop_assert!(outcomes.len() <= hook_picks.len());
        for o in &outcomes { prop_assert!(o.delta.len() <= 8); }
    }
}
