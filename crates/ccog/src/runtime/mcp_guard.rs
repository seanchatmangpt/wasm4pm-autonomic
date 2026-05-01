//! MCP Strips Guard: Precondition and effect validation for MCP tool calls.

use crate::compiled_hook::compute_present_mask;
use crate::runtime::cog8::Instinct;
use crate::runtime::mcp::MCPCall;
use crate::runtime::ClosedFieldContext;

/// Rejection reasons for an MCP tool call guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardRejection {
    /// Required variables are missing from the current context.
    MissingPrecondition,
    /// The effect of the tool call is not bounded within the allowed region.
    EffectUnbounded,
    /// The tool call schema or arguments are invalid.
    SchemaInvalid,
}

impl GuardRejection {
    /// Converts the rejection into a canonical instinct.
    pub fn to_instinct(&self) -> Instinct {
        match self {
            GuardRejection::MissingPrecondition => Instinct::Ask,
            GuardRejection::EffectUnbounded => Instinct::Refuse,
            GuardRejection::SchemaInvalid => Instinct::Refuse,
        }
    }
}

/// Guard that validates MCP tool calls against Strips-like preconditions.
pub struct StripsGuard;

impl StripsGuard {
    /// Evaluates whether an MCP tool call is admitted by the current context.
    ///
    /// Checks if the `required_vars` bitmask of the call is fully satisfied by the
    /// `present_mask` computed from the current snapshot.
    pub fn evaluate(call: &MCPCall, context: &ClosedFieldContext) -> Result<(), GuardRejection> {
        let present_mask = compute_present_mask(&context.snapshot);

        if (present_mask & call.required_vars) != call.required_vars {
            return Err(GuardRejection::MissingPrecondition);
        }

        // Phase 2.1 focuses on MissingPrecondition.
        // EffectUnbounded and SchemaInvalid are reserved for subsequent phases.

        Ok(())
    }
}
