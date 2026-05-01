//! ccog Adversarial Self-Play (ASP) Framework (PRD v0.9.5).
//!
//! Implements active adversarial search to maximize false-closure discovery.
//! The goal is to pressure-test lawful closure under hostile ambiguity,
//! conflicting evidence, and misleading process history.

use crate::runtime::{cog8::Instinct, ClosedFieldContext};
use anyhow::Result;

/// Adversarial Loss Components (L_adv).
#[derive(Debug, Clone, Default)]
pub struct AdversarialLoss {
    /// False Closure: ccog(O*) = A, but A is unlawful.
    pub false_closure: f64,
    /// Missed Closure: O* |- A, but ccog(O*) != A.
    pub missed_closure: f64,
    /// Overgeneralization: Executable(pi) but !Lawful(pi).
    pub overgeneralization: f64,
    /// Wrong Projection: correct A but wrong MCP/A2A target.
    pub wrong_projection: f64,
    /// Proof Gap: action emitted without Proof64.
    pub proof_gap: f64,
    /// Human Burden Violation: human overloaded but still asked.
    pub human_burden_violation: f64,
}

impl AdversarialLoss {
    /// Calculate total weighted loss.
    pub fn total(&self) -> f64 {
        self.false_closure * 10.0
            + self.missed_closure * 5.0
            + self.overgeneralization * 8.0
            + self.wrong_projection * 3.0
            + self.proof_gap * 7.0
            + self.human_burden_violation * 4.0
    }
}

/// Active Adversary that mutates scenarios to maximize loss.
pub trait CcogAdversary {
    /// Setup the initial field state for a scenario.
    fn setup_field(&self) -> Result<crate::field::FieldContext> {
        Ok(crate::field::FieldContext::new("asp-default"))
    }
    /// Mutate the field and context to create a "temptation" for false closure.
    fn mutate(&self, context: &mut ClosedFieldContext) -> Result<()>;
    /// Identify the expected (lawful) response for the mutated state.
    fn expected_response(&self, context: &ClosedFieldContext) -> Instinct;
}

/// ASP Tournament Orchestrator.
pub struct AdversarialTournament<A: CcogAdversary> {
    /// The active adversary searching for closure failures.
    pub adversary: A,
    /// Number of search iterations to perform.
    pub iterations: usize,
}

impl<A: CcogAdversary> AdversarialTournament<A> {
    /// Create a new adversarial tournament.
    pub fn new(adversary: A, iterations: usize) -> Self {
        Self {
            adversary,
            iterations,
        }
    }

    /// Run the tournament and return the scenario that caused the maximum loss.
    pub fn run(&self) -> Result<(f64, AdversarialLoss, String)> {
        let mut max_loss_val = -1.0;
        let mut worst_loss = AdversarialLoss::default();
        let mut worst_context_log = String::new();

        for _ in 0..self.iterations {
            // 1. Setup field via adversary
            let field = crate::field::FieldContext::new("asp-temp");
            let snap =
                std::sync::Arc::new(crate::compiled::CompiledFieldSnapshot::from_field(&field)?);
            let mut context = ClosedFieldContext {
                snapshot: snap,
                posture: crate::multimodal::PostureBundle::default(),
                context: crate::multimodal::ContextBundle::default(),
                tiers: crate::packs::TierMasks::ZERO,
                human_burden: 0,
            };

            // 2. Adversary Mutates (Search for a weak point)
            self.adversary.mutate(&mut context)?;
            let expected = self.adversary.expected_response(&context);

            // 3. Actor Decides
            let actual_instinct = crate::instinct::select_instinct_v0(&context);
            let actual: Instinct = actual_instinct.into();

            // 4. Calculate Loss
            let mut current_loss = AdversarialLoss::default();
            if actual != expected {
                // Any mismatch is a form of missed or false closure
                if (expected == Instinct::Ask || expected == Instinct::Retrieve)
                    && actual == Instinct::Ignore
                {
                    current_loss.missed_closure = 1.0;
                } else if expected != Instinct::Settle && actual == Instinct::Settle {
                    current_loss.false_closure = 1.0;
                } else {
                    // General missed closure for other mismatches
                    current_loss.missed_closure = 1.0;
                }
            }

            let total = current_loss.total();
            if total > max_loss_val {
                max_loss_val = total;
                worst_loss = current_loss;
                worst_context_log = format!("{:?}", context);
            }
        }

        Ok((max_loss_val, worst_loss, worst_context_log))
    }
}

/// Adversary that uses greedy search to maximize adversarial loss.
pub struct SearchAdversary {
    /// Number of greedy improvement steps.
    pub search_depth: usize,
}

impl SearchAdversary {
    /// Internal loss evaluator for the greedy search.
    fn evaluate_loss(&self, context: &ClosedFieldContext) -> f64 {
        let expected = self.expected_response(context);
        let actual: Instinct = crate::instinct::select_instinct_v0(context).into();

        if actual == expected {
            return 0.0;
        }

        let mut loss = AdversarialLoss::default();
        if (expected == Instinct::Ask || expected == Instinct::Retrieve)
            && actual == Instinct::Ignore
        {
            loss.missed_closure = 1.0;
        } else if expected != Instinct::Settle && actual == Instinct::Settle {
            loss.false_closure = 1.0;
        } else {
            loss.missed_closure = 1.0;
        }
        loss.total()
    }
}

impl CcogAdversary for SearchAdversary {
    fn setup_field(&self) -> Result<crate::field::FieldContext> {
        let mut field = crate::field::FieldContext::new("asp-stale");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        Ok(field)
    }

    fn mutate(&self, context: &mut ClosedFieldContext) -> Result<()> {
        let mut current_loss = self.evaluate_loss(context);

        for _ in 0..self.search_depth {
            let mut best_neighbor = context.clone();
            let mut best_loss = current_loss;
            let mut found_improvement = false;

            // Greedy search over posture bits (0-7 are core)
            for bit in 0..8 {
                let mut neighbor = context.clone();
                neighbor.posture.posture_mask ^= 1u64 << bit;
                let loss = self.evaluate_loss(&neighbor);
                if loss > best_loss {
                    best_loss = loss;
                    best_neighbor = neighbor;
                    found_improvement = true;
                }
            }

            // Greedy search over context expectation bits
            for bit in 0..8 {
                let mut neighbor = context.clone();
                neighbor.context.expectation_mask ^= 1u64 << bit;
                let loss = self.evaluate_loss(&neighbor);
                if loss > best_loss {
                    best_loss = loss;
                    best_neighbor = neighbor;
                    found_improvement = true;
                }
            }

            if found_improvement {
                *context = best_neighbor;
                current_loss = best_loss;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn expected_response(&self, context: &ClosedFieldContext) -> Instinct {
        let present = crate::compiled_hook::compute_present_mask(&context.snapshot);
        // If evidence is missing, ground truth is always Ask.
        if (present & (1u64 << crate::compiled_hook::Predicate::DD_MISSING_PROV_VALUE)) != 0 {
            return Instinct::Ask;
        }

        // Otherwise fallback to whatever the Actor would normally do in a clean state
        // (Simplified for the search)
        Instinct::Ignore
    }
}
