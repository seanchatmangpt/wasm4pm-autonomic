//! ccog Self-Play Loop Testing (PRD v0.9).
//!
//! Implements a multi-role simulation framework for stress-testing the entire
//! task ecology. Roles include Actor, Environment, Critic, Counterfactual, and Teacher.

use crate::construct8::Construct8;
use crate::powl64::Powl64;
use crate::runtime::{cog8::Cog8Decision, ClosedFieldContext};
use anyhow::Result;

/// Result of a single self-play step.
#[derive(Debug, Clone)]
pub struct SelfPlayStep {
    /// Decision made by the actor.
    pub decision: Cog8Decision,
    /// Route proof generated.
    pub proof: Powl64,
    /// Mutation applied to the field.
    pub delta: Construct8,
    /// Critic's verdict on this step.
    pub criticism: Option<String>,
}

/// Scenario configuration for self-play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioFamily {
    /// Expected successful closure.
    Normal,
    /// Document or fact missing.
    MissingEvidence,
    /// Conflicting data sources.
    ConflictingEvidence,
    /// Expired or outdated information.
    StaleEvidence,
    /// High-risk or policy-violating state.
    Risky,
    /// Redundant events.
    Duplicate,
}

/// Core trait for self-play environmental simulation.
pub trait CcogEnvironment {
    /// Generate an initial field state for the given scenario family.
    fn setup(&mut self, family: ScenarioFamily) -> Result<()>;
    /// Observe an action and update the field state (simulation step).
    fn step(&mut self, decision: &Cog8Decision) -> Result<()>;
    /// Get the current closed field context.
    fn context(&self) -> ClosedFieldContext;
}

/// Core trait for self-play criticism and lawfulness checking.
pub trait CcogCritic {
    /// Evaluate whether the decision and route were lawful under the context.
    fn critique(
        &self,
        context: &ClosedFieldContext,
        decision: &Cog8Decision,
        proof: &Powl64,
    ) -> Result<()>;
}

/// Core trait for counterfactual mutation (adversarial simulation).
pub trait CcogCounterfactual {
    /// Mutate the context to test edge cases (e.g., "what if this human was slow?").
    fn mutate(&self, context: &mut ClosedFieldContext) -> Result<()>;
}

/// The Self-Play Loop Orchestrator.
pub struct SelfPlayLoop<E: CcogEnvironment, C: CcogCritic, CF: CcogCounterfactual> {
    /// Simulated process ecology environment.
    pub env: E,
    /// Lawfulness and closure critic.
    pub critic: C,
    /// Adversarial state mutator.
    pub counterfactual: CF,
    /// History of steps in the current run.
    pub steps: Vec<SelfPlayStep>,
}

impl<E: CcogEnvironment, C: CcogCritic, CF: CcogCounterfactual> SelfPlayLoop<E, C, CF> {
    /// Create a new self-play loop with given participants.
    pub fn new(env: E, critic: C, counterfactual: CF) -> Self {
        Self {
            env,
            critic,
            counterfactual,
            steps: Vec::new(),
        }
    }

    /// Run a full self-play tournament for a scenario family.
    pub fn run(&mut self, family: ScenarioFamily, max_steps: usize) -> Result<()> {
        self.env.setup(family)?;

        for _ in 0..max_steps {
            let mut ctx = self.env.context();

            // Apply counterfactual stress
            self.counterfactual.mutate(&mut ctx)?;

            // 1. Actor Decide (Using the default instinct selection for now)
            let decision = crate::instinct::select_instinct_v0(&ctx);
            // Convert AutonomicInstinct to Cog8Decision (placeholder wrapper)
            let cog8_decision = Cog8Decision {
                response: decision.into(),
                ..Default::default()
            };

            // 2. Criticize
            let proof = Powl64::default(); // Placeholder
            self.critic.critique(&ctx, &cog8_decision, &proof)?;

            // 3. Env Step
            self.env.step(&cog8_decision)?;

            self.steps.push(SelfPlayStep {
                decision: cog8_decision,
                proof,
                delta: Construct8::empty(),
                criticism: None,
            });

            // Termination conditions
            if cog8_decision.response == crate::runtime::cog8::Instinct::Settle
                || cog8_decision.response == crate::runtime::cog8::Instinct::Refuse
            {
                break;
            }
        }

        Ok(())
    }
}
