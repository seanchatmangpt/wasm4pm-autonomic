//! Agent-to-Agent (A2A) Core Layer (Phase 2).
//!
//! Provides the projection logic from COG8 decisions to external agent tasks.

use crate::runtime::cog8::{AgentId, Cog8Decision, Instinct};
use crate::runtime::ClosedFieldContext;

/// Profile of capabilities supported by an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityProfile {
    /// Agent can perform reasoning tasks.
    Reasoning,
    /// Agent can perform search tasks.
    Search,
    /// Agent can perform coordination tasks.
    Coordination,
}

/// Template for projecting a COG8 decision into an A2A task.
#[derive(Debug, Clone, Copy)]
pub struct A2ATaskTemplate {
    /// Unique identifier for the target agent.
    pub agent_id: AgentId,
    /// Required capability for the task.
    pub required_capability: CapabilityProfile,
    /// Bitmask of required variables from the context (u64 mask).
    pub required_vars: u64,
}

/// A specific agent task instance produced by the projection logic.
#[derive(Debug, Clone)]
pub struct A2ATask {
    /// The agent to task.
    pub agent_id: AgentId,
    /// Bitmask of required variables from the context.
    pub required_vars: u64,
    /// Task-specific parameters.
    pub parameters: A2AParameters,
}

/// Compact parameter representation for zero-allocation agent tasks.
#[derive(Debug, Clone, Default)]
pub struct A2AParameters {
    /// Opaque handle or primary parameter.
    pub handle: u64,
    /// Secondary context parameter.
    pub context_ref: u64,
}

/// Rule mapping an instinct response to an A2A task template.
#[derive(Debug, Clone, Copy)]
pub struct A2AProjectionRule {
    /// The instinct that triggers this projection.
    pub trigger_instinct: Instinct,
    /// Optional collapse function to match.
    pub collapse_fn: Option<crate::ids::CollapseFn>,
    /// The template for the resulting task.
    pub template: A2ATaskTemplate,
}

/// Table of projection rules for A2A integration.
#[derive(Debug, Clone, Copy)]
pub struct A2AProjectionTable {
    /// Set of projection rules.
    pub rules: &'static [A2AProjectionRule],
}

impl A2AProjectionTable {
    /// Project a COG8 decision into an optional A2A task.
    ///
    /// This evaluates the decision against the projection table to determine
    /// if an external agent should be tasked.
    pub fn project(
        &self,
        decision: &Cog8Decision,
        _context: &ClosedFieldContext,
    ) -> Option<A2ATask> {
        for rule in self.rules {
            if rule.trigger_instinct == decision.response {
                // If the rule specifies a collapse_fn, it must match.
                if let Some(cf) = rule.collapse_fn {
                    if decision.collapse_fn != Some(cf) {
                        continue;
                    }
                }

                return Some(A2ATask {
                    agent_id: rule.template.agent_id,
                    required_vars: rule.template.required_vars,
                    parameters: A2AParameters::default(),
                });
            }
        }
        None
    }
}
