//! Model Context Protocol (MCP) Runtime Integration (Phase 1.1/1.2).
//!
//! Provides the projection logic from COG8 decisions to external MCP tool calls.

use crate::runtime::cog8::{Cog8Decision, Instinct};
use crate::runtime::ClosedFieldContext;

/// Model Context Protocol Tool Identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolId(pub u16);

/// Result type expectation for an MCP tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedResultType {
    /// No result expected.
    None,
    /// Expects a Construct8 result.
    Construct8,
    /// Expects a TruthBlock (L1Region).
    TruthBlock,
    /// Expects raw octets.
    Octets,
}

/// Effect policy for an MCP tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPolicy {
    /// Pure observation, no side effects.
    Read,
    /// Mutates external state.
    Write,
    /// Full administrative control.
    Admin,
}

/// Template for projecting a COG8 decision into an MCP tool call.
#[derive(Debug, Clone, Copy)]
pub struct ToolCallTemplate {
    /// Unique identifier for the tool.
    pub tool_id: ToolId,
    /// Expected response format.
    pub expected_result_type: ExpectedResultType,
    /// Permission level for the call.
    pub effect_policy: EffectPolicy,
    /// Bitmask of required variables from the context (u64 mask).
    pub required_vars: u64,
}

/// A specific tool call instance produced by the projection logic.
#[derive(Debug, Clone)]
pub struct MCPCall {
    /// The tool to invoke.
    pub tool_id: ToolId,
    /// The collapse function validating this call.
    pub collapse_fn: crate::ids::CollapseFn,
    /// Bitmask of required variables from the context.
    pub required_vars: u64,
    /// Compact argument data.
    pub arguments: MCPArguments,
}

/// Compact argument representation for zero-allocation tool calls.
#[derive(Debug, Clone, Default)]
pub struct MCPArguments {
    /// Optional primary parameter.
    pub param0: u64,
    /// Optional secondary parameter.
    pub param1: u64,
}

/// Rule mapping an instinct response to a tool call template.
#[derive(Debug, Clone, Copy)]
pub struct ProjectionRule {
    /// The instinct that triggers this projection.
    pub trigger_instinct: Instinct,
    /// Optional collapse function to match.
    pub collapse_fn: Option<crate::ids::CollapseFn>,
    /// The template for the resulting tool call.
    pub template: ToolCallTemplate,
}

/// Table of projection rules for MCP integration.
#[derive(Debug, Clone, Copy)]
pub struct MCPProjectionTable {
    /// Set of projection rules.
    pub rules: &'static [ProjectionRule],
}

impl Default for MCPProjectionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPProjectionTable {
    /// Create a new, empty projection table.
    pub const fn new() -> Self {
        Self { rules: &[] }
    }

    /// Project a COG8 decision into an optional MCP tool call.
    ///
    /// This evaluates the decision against the projection table to determine
    /// if an external tool should be invoked.
    pub fn project(
        &self,
        decision: &Cog8Decision,
        _context: &ClosedFieldContext,
    ) -> Option<MCPCall> {
        // Iterate through rules to find a match for the decision's response.
        for rule in self.rules {
            if rule.trigger_instinct == decision.response {
                let cf = if let Some(cf) = rule.collapse_fn {
                    if decision.collapse_fn != Some(cf) {
                        continue;
                    }
                    cf
                } else {
                    if let Some(cf) = decision.collapse_fn {
                        cf
                    } else {
                        continue;
                    }
                };

                return Some(MCPCall {
                    tool_id: rule.template.tool_id,
                    collapse_fn: cf,
                    required_vars: rule.template.required_vars,
                    arguments: MCPArguments::default(),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{BreedId, CollapseFn, EdgeId, GroupId, NodeId, PackId, RuleId};
    use crate::runtime::cog8::{Cog8Decision, Instinct};

    static TEST_RULES: [ProjectionRule; 1] = [ProjectionRule {
        trigger_instinct: Instinct::Retrieve,
        collapse_fn: Some(CollapseFn::Grounding),
        template: ToolCallTemplate {
            tool_id: ToolId(42),
            expected_result_type: ExpectedResultType::Construct8,
            effect_policy: EffectPolicy::Write,
            required_vars: 0xFF,
        },
    }];

    #[test]
    fn test_mcp_projection() {
        let table = MCPProjectionTable { rules: &TEST_RULES };

        let decision = Cog8Decision {
            response: Instinct::Retrieve,
            matched_pack_id: Some(PackId(1)),
            matched_group_id: Some(GroupId(2)),
            matched_rule_id: Some(RuleId(3)),
            matched_breed_id: Some(BreedId(4)),
            collapse_fn: Some(CollapseFn::Grounding),
            selected_node: Some(NodeId(5)),
            selected_edge: Some(EdgeId(6)),
            completed_mask: 0,
            fired_mask: 0,
            denied_mask: 0,
        };

        // Minimal context for projection
        let field = crate::field::FieldContext::new("test");
        let snap = crate::compiled::CompiledFieldSnapshot::from_field(&field).unwrap();
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: crate::multimodal::PostureBundle::default(),
            context: crate::multimodal::ContextBundle::default(),
            tiers: crate::packs::TierMasks::ZERO,
            human_burden: 0,
        };

        let call = table.project(&decision, &context);
        assert!(call.is_some());
        assert_eq!(call.unwrap().tool_id.0, 42);
    }
}
