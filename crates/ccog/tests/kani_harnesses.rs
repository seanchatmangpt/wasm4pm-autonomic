use ccog::ids::*;
use ccog::runtime::cog8::*;

#[cfg(kani)]
#[kani::proof]
fn kani_execute_cog8_graph_never_panics() {
    let present: u64 = kani::any();
    let completed: u64 = kani::any();

    let node = Cog8Row {
        pack_id: PackId(kani::any()),
        group_id: GroupId(kani::any()),
        rule_id: RuleId(kani::any()),
        breed_id: BreedId(kani::any()),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: kani::any(),
        forbidden_mask: kani::any(),
        predecessor_mask: kani::any(),
        response: Instinct::Settle,
        priority: kani::any(),
    };

    let edge = Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(0),
            guard_mask: kani::any(),
            effect_mask: kani::any(),
        },
    };

    let nodes = [node];
    let edges = [edge];

    let _ = execute_cog8_graph(&nodes, &edges, present, completed);
}
