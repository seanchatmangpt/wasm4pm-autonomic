use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::ids::{BreedId, EdgeId, GroupId, NodeId, PackId, RuleId};
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::runtime::cog8::{Cog8Decision, Instinct};
use ccog::runtime::mcp::{
    EffectPolicy, ExpectedResultType, MCPProjectionTable, ProjectionRule, ToolCallTemplate, ToolId,
};
use ccog::runtime::ClosedFieldContext;
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

static TEST_RULES: [ProjectionRule; 1] = [ProjectionRule {
    trigger_instinct: Instinct::Retrieve,
    collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
    template: ToolCallTemplate {
        tool_id: ToolId(42),
        expected_result_type: ExpectedResultType::Construct8,
        effect_policy: EffectPolicy::Write,
        required_vars: 0xFF,
    },
}];

#[test]
fn test_mcp_projection_positive() {
    let table = MCPProjectionTable { rules: &TEST_RULES };

    let decision = Cog8Decision {
        response: Instinct::Retrieve,
        matched_pack_id: Some(PackId(1)),
        matched_group_id: Some(GroupId(2)),
        matched_rule_id: Some(RuleId(3)),
        matched_breed_id: Some(BreedId(4)),
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        selected_node: Some(NodeId(5)),
        selected_edge: Some(EdgeId(6)),
        completed_mask: 0,
        fired_mask: 0,
        denied_mask: 0,
    };

    let field = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap);

    let call = table.project(&decision, &context).expect("Should project");
    assert_eq!(call.tool_id, ToolId(42));
    assert_eq!(call.required_vars, 0xFF);
}

#[test]
fn test_mcp_projection_negative_mismatch() {
    let table = MCPProjectionTable { rules: &TEST_RULES };

    let decision = Cog8Decision {
        response: Instinct::Ask, // Different instinct
        matched_pack_id: Some(PackId(1)),
        matched_group_id: Some(GroupId(2)),
        matched_rule_id: Some(RuleId(3)),
        matched_breed_id: Some(BreedId(4)),
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        selected_node: Some(NodeId(5)),
        selected_edge: Some(EdgeId(6)),
        completed_mask: 0,
        fired_mask: 0,
        denied_mask: 0,
    };

    let field = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap);

    let call = table.project(&decision, &context);
    assert!(call.is_none());
}

static MULTI_RULES: [ProjectionRule; 2] = [
    ProjectionRule {
        trigger_instinct: Instinct::Retrieve,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        template: ToolCallTemplate {
            tool_id: ToolId(1),
            expected_result_type: ExpectedResultType::Construct8,
            effect_policy: EffectPolicy::Write,
            required_vars: 0x1,
        },
    },
    ProjectionRule {
        trigger_instinct: Instinct::Ask,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        template: ToolCallTemplate {
            tool_id: ToolId(2),
            expected_result_type: ExpectedResultType::TruthBlock,
            effect_policy: EffectPolicy::Read,
            required_vars: 0x2,
        },
    },
];

#[test]
fn test_mcp_projection_multiple_rules() {
    let table = MCPProjectionTable {
        rules: &MULTI_RULES,
    };

    let field = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap);

    let d_retrieve = Cog8Decision {
        response: Instinct::Retrieve,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };
    let call_retrieve = table.project(&d_retrieve, &context).unwrap();
    assert_eq!(call_retrieve.tool_id, ToolId(1));

    let d_ask = Cog8Decision {
        response: Instinct::Ask,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };
    let call_ask = table.project(&d_ask, &context).unwrap();
    assert_eq!(call_ask.tool_id, ToolId(2));
}

#[test]
fn test_mcp_projection_perturbation_empty_table() {
    let table = MCPProjectionTable { rules: &[] };
    let decision = Cog8Decision {
        response: Instinct::Retrieve,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };
    let field = FieldContext::new("test");
    let snap = Arc::new(CompiledFieldSnapshot::from_field(&field).unwrap());
    let context = empty_context(snap);

    assert!(table.project(&decision, &context).is_none());
}
