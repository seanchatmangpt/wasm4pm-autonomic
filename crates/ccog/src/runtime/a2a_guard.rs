//! A2A Strips Guard: Precondition and effect validation for A2A tasks.

use crate::compiled_hook::compute_present_mask;
use crate::runtime::a2a::A2ATask;
use crate::runtime::cog8::Instinct;
use crate::runtime::ClosedFieldContext;

/// Rejection reasons for an A2A task guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardRejection {
    /// Required variables are missing from the current context.
    MissingPrecondition,
    /// The effect of the task is not bounded.
    EffectUnbounded,
    /// The task capability requirements are not met.
    CapabilityMismatch,
}

impl GuardRejection {
    /// Converts the rejection into a canonical instinct.
    pub fn to_instinct(&self) -> Instinct {
        match self {
            GuardRejection::MissingPrecondition => Instinct::Ask,
            GuardRejection::EffectUnbounded => Instinct::Refuse,
            GuardRejection::CapabilityMismatch => Instinct::Refuse,
        }
    }
}

/// Guard that validates A2A tasks against Strips-like preconditions.
pub struct StripsGuard;

impl StripsGuard {
    /// Evaluates whether an A2A task is admitted by the current context.
    pub fn evaluate(task: &A2ATask, context: &ClosedFieldContext) -> Result<(), GuardRejection> {
        let present_mask = compute_present_mask(&context.snapshot);

        if (present_mask & task.required_vars) != task.required_vars {
            return Err(GuardRejection::MissingPrecondition);
        }

        Ok(())
    }
}
