use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
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

static DELEGATION_RULES: [ProjectionRule; 1] = [ProjectionRule {
    trigger_instinct: Instinct::Escalate,
    collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
    template: ToolCallTemplate {
        tool_id: ToolId(101), // mcp:tool:delegate_to_agent
        expected_result_type: ExpectedResultType::Construct8,
        effect_policy: EffectPolicy::Write,
        required_vars: 0x0,
    },
}];

/// A2A Delegation Test:
/// Tests the pattern where a decision to Escalate is projected into an MCP
/// tool call that represents delegating the task to another agent.
#[test]
fn test_a2a_delegation_via_mcp_projection() {
    let table = MCPProjectionTable {
        rules: &DELEGATION_RULES,
    };

    // Simulate a decision that triggers escalation.
    let decision = Cog8Decision {
        response: Instinct::Escalate,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };

    let field = FieldContext::new("a2a-test");
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let context = empty_context(std::sync::Arc::new(snap.clone()));

    let call = table
        .project(&decision, &context)
        .expect("Escalate should project to delegation call");
    assert_eq!(call.tool_id, ToolId(101));
}

static MULTI_DELEGATION_RULES: [ProjectionRule; 2] = [
    ProjectionRule {
        trigger_instinct: Instinct::Escalate,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        template: ToolCallTemplate {
            tool_id: ToolId(101),
            expected_result_type: ExpectedResultType::Construct8,
            effect_policy: EffectPolicy::Write,
            required_vars: 0x0,
        },
    },
    ProjectionRule {
        trigger_instinct: Instinct::Settle,
        collapse_fn: None,
        template: ToolCallTemplate {
            tool_id: ToolId(202),
            expected_result_type: ExpectedResultType::None,
            effect_policy: EffectPolicy::Read,
            required_vars: 0x0,
        },
    },
];

/// Metamorphic Invariant:
/// Adding unrelated rules to the projection table should not change the delegation outcome.
#[test]
fn test_a2a_delegation_metamorphic_invariant() {
    let table_a = MCPProjectionTable {
        rules: &DELEGATION_RULES,
    };

    let table_b = MCPProjectionTable {
        rules: &MULTI_DELEGATION_RULES,
    };

    let decision = Cog8Decision {
        response: Instinct::Escalate,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };

    let field = FieldContext::new("a2a-test");
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let context = empty_context(std::sync::Arc::new(snap.clone()));

    let call_a = table_a.project(&decision, &context).unwrap();
    let call_b = table_b.project(&decision, &context).unwrap();

    assert_eq!(call_a.tool_id, call_b.tool_id);
}

/// Perturbation Invariant:
/// Removing the delegation rule should result in no delegation call being produced.
#[test]
fn test_a2a_delegation_perturbation_invariant() {
    let table = MCPProjectionTable {
        rules: &DELEGATION_RULES,
    };

    let decision = Cog8Decision {
        response: Instinct::Escalate,
        collapse_fn: Some(ccog::ids::CollapseFn::ExpertRule),
        ..Default::default()
    };

    let field = FieldContext::new("a2a-test");
    let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
    let context = empty_context(std::sync::Arc::new(snap.clone()));
    assert!(table.project(&decision, &context).is_some());

    // Perturb: Remove the rule.
    let table_perturbed = MCPProjectionTable { rules: &[] };
    assert!(table_perturbed.project(&decision, &context).is_none());
}
