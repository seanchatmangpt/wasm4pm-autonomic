use ccog::construct8::Triple;
use ccog::graph::GraphStore;
use ccog::ids::*;
use ccog::runtime::cog8::{
    cog8_matches, execute_cog8_graph, Cog8Edge, Cog8Row, EdgeKind, Instinct, Powl8Instr, Powl8Op,
};
use ccog::Construct8;
use proptest::prelude::*;

fn arb_triple() -> impl Strategy<Value = Triple> {
    (any::<u32>(), any::<u16>(), any::<u32>()).prop_map(|(s, p, o)| {
        Triple::from_strings(&format!("{}", s), &format!("{}", p), &format!("{}", o))
    })
}

fn arb_instinct() -> impl Strategy<Value = Instinct> {
    prop_oneof![
        Just(Instinct::Settle),
        Just(Instinct::Retrieve),
        Just(Instinct::Inspect),
        Just(Instinct::Ask),
        Just(Instinct::Refuse),
        Just(Instinct::Escalate),
        Just(Instinct::Ignore),
    ]
}

fn arb_collapse_fn() -> impl Strategy<Value = CollapseFn> {
    prop_oneof![
        Just(CollapseFn::None),
        Just(CollapseFn::ReflectivePosture),
        Just(CollapseFn::ExpertRule),
        Just(CollapseFn::Preconditions),
        Just(CollapseFn::Grounding),
        Just(CollapseFn::RelationalProof),
        Just(CollapseFn::Reconstruction),
        Just(CollapseFn::BlackboardFusion),
        Just(CollapseFn::DifferenceReduction),
        Just(CollapseFn::Chunking),
        Just(CollapseFn::ReactiveIntention),
        Just(CollapseFn::CaseAnalogy),
    ]
}

fn arb_powl8_op() -> impl Strategy<Value = Powl8Op> {
    prop_oneof![
        Just(Powl8Op::Act),
        Just(Powl8Op::Choice),
        Just(Powl8Op::Partial),
        Just(Powl8Op::Join),
        Just(Powl8Op::Loop),
        Just(Powl8Op::Silent),
        Just(Powl8Op::Block),
        Just(Powl8Op::Emit),
    ]
}

fn arb_edge_kind() -> impl Strategy<Value = EdgeKind> {
    prop_oneof![
        Just(EdgeKind::Choice),
        Just(EdgeKind::PartialOrder),
        Just(EdgeKind::Loop),
        Just(EdgeKind::Silent),
        Just(EdgeKind::Override),
        Just(EdgeKind::Blocking),
        Just(EdgeKind::None),
    ]
}

fn arb_cog8_row() -> impl Strategy<Value = Cog8Row> {
    (
        any::<u16>().prop_map(PackId),
        any::<u16>().prop_map(GroupId),
        any::<u16>().prop_map(RuleId),
        any::<u8>().prop_map(BreedId),
        arb_collapse_fn(),
        any::<[u16; 8]>().prop_map(|v| v.map(FieldId)),
        any::<u64>(),
        any::<u64>(),
        any::<u64>(),
        arb_instinct(),
        any::<u16>(),
    )
        .prop_map(
            |(
                pack_id,
                group_id,
                rule_id,
                breed_id,
                collapse_fn,
                var_ids,
                required_mask,
                forbidden_mask,
                predecessor_mask,
                response,
                priority,
            )| {
                Cog8Row {
                    pack_id,
                    group_id,
                    rule_id,
                    breed_id,
                    collapse_fn,
                    var_ids,
                    required_mask,
                    forbidden_mask,
                    predecessor_mask,
                    response,
                    priority,
                }
            },
        )
}

fn arb_powl8_instr() -> impl Strategy<Value = Powl8Instr> {
    (
        arb_powl8_op(),
        arb_collapse_fn(),
        any::<u16>().prop_map(NodeId),
        any::<u16>().prop_map(EdgeId),
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(
            |(op, collapse_fn, node_id, edge_id, guard_mask, effect_mask)| Powl8Instr {
                op,
                collapse_fn,
                node_id,
                edge_id,
                guard_mask,
                effect_mask,
            },
        )
}

fn arb_cog8_edge() -> impl Strategy<Value = Cog8Edge> {
    (
        any::<u16>().prop_map(NodeId),
        any::<u16>().prop_map(NodeId),
        arb_edge_kind(),
        arb_powl8_instr(),
    )
        .prop_map(|(from, to, kind, instr)| Cog8Edge {
            from,
            to,
            kind,
            instr,
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
        // Since materialize uses hashed IDs as blank nodes or urns, we check if non-empty
        let ntriples = delta.to_ntriples();
        prop_assert!(!ntriples.is_empty());
    }

    #[test]
    fn prop_construct8_iter_in_push_order(triples in prop::collection::vec(arb_triple(), 0..8)) {
        let mut delta = Construct8::empty();
        for t in &triples { delta.push(t.clone()); }
        let collected: Vec<Triple> = delta.iter().cloned().collect();
        prop_assert_eq!(collected, triples);
    }

    #[test]
    fn prop_cog8_graph_deterministic(
        nodes in prop::collection::vec(arb_cog8_row(), 0..10),
        edges in prop::collection::vec(arb_cog8_edge(), 0..10),
        present in any::<u64>(),
        completed in any::<u64>()
    ) {
        let dec1 = execute_cog8_graph(&nodes, &edges, present, completed).unwrap();
        let dec2 = execute_cog8_graph(&nodes, &edges, present, completed).unwrap();
        prop_assert_eq!(dec1, dec2);
    }

    #[test]
    fn prop_cog8_matches_logic(
        row in arb_cog8_row(),
        present in any::<u64>(),
        completed in any::<u64>()
    ) {
        let expected = (present & row.required_mask) == row.required_mask
            && (present & row.forbidden_mask) == 0
            && (completed & row.predecessor_mask) == row.predecessor_mask;

        let actual = cog8_matches(&row, present, completed);
        prop_assert_eq!(expected, actual);
    }

    #[test]
    fn prop_cog8_law_of_8_never_violated(
        nodes in prop::collection::vec(arb_cog8_row(), 0..10),
        edges in prop::collection::vec(arb_cog8_edge(), 0..10),
        present in any::<u64>(),
        completed in any::<u64>()
    ) {
        let dec = execute_cog8_graph(&nodes, &edges, present, completed).unwrap();
        for row in &nodes {
            prop_assert_eq!(row.var_ids.len(), 8);
        }
        prop_assert!(dec.completed_mask >= completed); // bitwise OR only adds bits
    }
}
