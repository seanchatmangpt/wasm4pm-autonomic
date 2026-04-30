use proptest::prelude::*;
use ccog::Construct8;
use ccog::graph::GraphStore;
use oxigraph::model::{NamedNode, Term, Triple};

fn arb_iri() -> impl Strategy<Value = String> {
    "[a-z]{3,8}".prop_map(|s| format!("http://example.org/{}", s))
}

fn arb_triple() -> impl Strategy<Value = Triple> {
    (arb_iri(), arb_iri(), arb_iri()).prop_map(|(s, p, o)| {
        Triple::new(
            NamedNode::new(&s).unwrap(),
            NamedNode::new(&p).unwrap(),
            Term::NamedNode(NamedNode::new(&o).unwrap()),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_construct8_never_exceeds_eight(triples in prop::collection::vec(arb_triple(), 0..20)) {
        let mut delta = Construct8::empty();
        for (i, t) in triples.iter().enumerate() {
            let pushed = delta.push(t.clone());
            if i < 8 { prop_assert!(pushed); } else { prop_assert!(!pushed); }
        }
        prop_assert!(delta.len() <= 8);
    }

    #[test]
    fn prop_construct8_receipt_bytes_deterministic(triples in prop::collection::vec(arb_triple(), 0..8)) {
        let mut a = Construct8::empty();
        let mut b = Construct8::empty();
        for t in &triples { a.push(t.clone()); b.push(t.clone()); }
        prop_assert_eq!(a.receipt_bytes(), b.receipt_bytes());
    }

    #[test]
    fn prop_construct8_materialize_round_trip(triples in prop::collection::vec(arb_triple(), 1..8)) {
        let mut delta = Construct8::empty();
        for t in &triples { delta.push(t.clone()); }
        let store = GraphStore::new();
        delta.materialize(&store).unwrap();
        let rows = store.select("SELECT ?s WHERE { ?s ?p ?o }").unwrap();
        prop_assert!(rows.len() <= delta.len());
        prop_assert!(!rows.is_empty());
    }

    #[test]
    fn prop_construct8_iter_in_push_order(triples in prop::collection::vec(arb_triple(), 0..8)) {
        let mut delta = Construct8::empty();
        for t in &triples { delta.push(t.clone()); }
        let collected: Vec<Triple> = delta.iter().cloned().collect();
        prop_assert_eq!(collected, triples);
    }
}
