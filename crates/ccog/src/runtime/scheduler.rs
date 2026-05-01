//! Hook scheduler — drives the closed-loop tick and fires hooks on detected ΔO.

use crate::field::FieldContext;
use crate::hooks::{HookOutcome, HookRegistry};
use crate::runtime::delta::{GraphDelta, GraphSnapshot};
use crate::runtime::error::{Result, RuntimeError};

/// Scheduler that detects ΔO between ticks and fires registered hooks on change.
#[derive(Debug)]
pub struct Scheduler {
    registry: HookRegistry,
    last_snapshot: Option<GraphSnapshot>,
}

/// Outcome of a single `tick`: the observed delta and any hook outcomes produced.
#[derive(Debug)]
pub struct TickReport {
    /// Delta computed against the prior snapshot (or `all_added` on the first tick).
    pub delta: GraphDelta,
    /// Hook outcomes from `registry.fire_matching` — empty when delta is empty after the first tick.
    pub outcomes: Vec<HookOutcome>,
}

impl Scheduler {
    /// Create a scheduler with the given registry and no prior snapshot.
    pub fn new(registry: HookRegistry) -> Self {
        Self {
            registry,
            last_snapshot: None,
        }
    }

    /// Run one tick: capture state, diff against prior snapshot, fire hooks if ΔO ≠ ∅.
    pub fn tick(&mut self, field: &FieldContext) -> Result<TickReport> {
        let current = GraphSnapshot::capture(&field.graph)
            .map_err(|e| RuntimeError::GraphError(e.to_string()))?;
        let delta = match &self.last_snapshot {
            Some(prev) => GraphDelta::between(prev, &current),
            None => GraphDelta::all_added(&current),
        };
        let outcomes = if delta.is_empty() && self.last_snapshot.is_some() {
            Vec::new()
        } else {
            self.registry
                .fire_matching(field)
                .map_err(|e| RuntimeError::HookError(e.to_string()))?
        };
        self.last_snapshot = Some(current);
        Ok(TickReport { delta, outcomes })
    }
}
