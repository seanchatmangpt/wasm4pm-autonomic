use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::runtime::cog8::{Cog8Decision, Instinct};
use ccog::runtime::mcp::{
    EffectPolicy, ExpectedResultType, MCPProjectionTable, ProjectionRule, ToolCallTemplate, ToolId,
};
use ccog::runtime::ClosedFieldContext;
use oxigraph::model::NamedNode;
use std::sync::Arc;

fn empty_context(snap: Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
    ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    }
}

static ASK_RULES: [ProjectionRule; 1] = [ProjectionRule {
    trigger_instinct: Instinct::Ask,
    collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
    template: ToolCallTemplate {
        tool_id: ToolId(303), // mcp:tool:ask_human_operator
        expected_result_type: ExpectedResultType::Construct8,
        effect_policy: EffectPolicy::Read,
        required_vars: 0x0,
    },
}];

/// HITL Role Test:
/// Tests the pattern where a decision to Ask is projected into an MCP
/// tool call that includes human role identification from the graph.
#[test]
fn test_hitl_role_consultation_via_mcp_projection() {
    let table = MCPProjectionTable { rules: &ASK_RULES };

    // Simulate a decision that triggers a request for clarification.
    let decision = Cog8Decision {
        response: Instinct::Ask,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };

    let mut field = FieldContext::new("hitl-test");
    // Add a role to the graph.
    field.load_field_state(
        "<http://example.org/op1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Role> .\n\
         <http://example.org/op1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"Senior Reviewer\" .\n"
    ).unwrap();

    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap.clone());
    // Verify role exists in the snapshot.
    let role_type = NamedNode::new("urn:ccog:Role").unwrap();
    let operators = snap.instances_of(&role_type);
    assert!(!operators.is_empty());

    let call = table
        .project(&decision, &context)
        .expect("Ask should project to HITL call");
    assert_eq!(call.tool_id, ToolId(303));
}

/// Metamorphic Invariant:
/// Adding more roles should still project to the same tool ID (lexicographical selection happens inside the tool/breed).
#[test]
fn test_hitl_role_metamorphic_invariant() {
    let table = MCPProjectionTable { rules: &ASK_RULES };

    let decision = Cog8Decision {
        response: Instinct::Ask,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };

    let mut field_a = FieldContext::new("hitl-test-a");
    field_a.load_field_state("<http://example.org/op1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Role> .\n").unwrap();
    let snap_a = Arc::new(CompiledFieldSnapshot::from_field(&field_a).unwrap());
    let context_a = empty_context(snap_a.clone());

    let mut field_b = FieldContext::new("hitl-test-b");
    field_b.load_field_state(
        "<http://example.org/op1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Role> .\n\
         <http://example.org/op2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Role> .\n"
    ).unwrap();
    let snap_b = Arc::new(CompiledFieldSnapshot::from_field(&field_b).unwrap());
    let context_b = empty_context(snap_b.clone());

    let call_a = table.project(&decision, &context_a).unwrap();
    let call_b = table.project(&decision, &context_b).unwrap();

    assert_eq!(call_a.tool_id, call_b.tool_id);
}

/// Perturbation Invariant:
/// If the decision is changed to Ignore, no HITL call should be produced.
#[test]
fn test_hitl_role_perturbation_invariant() {
    let table = MCPProjectionTable { rules: &ASK_RULES };

    let mut field = FieldContext::new("hitl-test");
    field.load_field_state("<http://example.org/op1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Role> .\n").unwrap();
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap.clone());

    let decision_ask = Cog8Decision {
        response: Instinct::Ask,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };
    assert!(table.project(&decision_ask, &context).is_some());

    let decision_ignore = Cog8Decision {
        response: Instinct::Ignore,
        ..Default::default()
    };
    assert!(table.project(&decision_ignore, &context).is_none());
}

#[test]
fn test_human_role_burden_and_selection() {
    use ccog::ids::HumanRoleId;
    use ccog::runtime::hitl::{HumanRoleProfile, LeastCostHandler};

    let mut op1 = HumanRoleProfile::new(HumanRoleId(1), 10, 0.9);
    let op2 = HumanRoleProfile::new(HumanRoleId(2), 5, 0.7);

    // Initial state: both have 0 burden. op1 should be selected due to higher reliability.
    let profiles = [op1, op2];
    let selected = LeastCostHandler::select(&profiles).unwrap();
    assert_eq!(selected.id, HumanRoleId(1));

    // Increase burden on op1.
    op1.add_burden(100);
    let profiles = [op1, op2];
    let selected = LeastCostHandler::select(&profiles).unwrap();
    assert_eq!(selected.id, HumanRoleId(2)); // op2 has lower burden now.

    // Apply decay to op1.
    op1.decay_burden(10, 10); // 10 ticks * 10 decay = 100 decay.
    assert_eq!(op1.current_burden, 0);

    let profiles = [op1, op2];
    let selected = LeastCostHandler::select(&profiles).unwrap();
    assert_eq!(selected.id, HumanRoleId(1)); // op1 back to 0 burden and higher reliability.
}
