//! High-concurrency tournament orchestration and scorecard reporting.
//!
//! Implements the `MaximumConcurrencyTournament` orchestrator supporting
//! scaled execution modes from single-core to distributed field swarms.
//! Reports performance, correctness, and ecology metrics via `AuditScorecard`.

pub mod cold_arena;
pub mod l1_arena;
pub mod l2_arena;
pub mod l3_arena;

pub use cold_arena::ColdEvidenceArena;
pub use l1_arena::L1ReflexArena;
pub use l2_arena::L2WorkingSkillArena;

use crate::runtime::cog8::Instinct;
use crate::runtime::self_play::{CcogCounterfactual, CcogCritic, CcogEnvironment, ScenarioFamily};
use anyhow::Result;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

/// Execution modes for the Maximum Concurrency Tournament.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TournamentMode {
    /// Mode A: Single-core, sequential execution.
    SingleCore,
    /// Mode B: Multi-core, parallel local execution.
    MultiCore,
    /// Mode C: Cluster-scale distributed execution (Simulated in local environment).
    Cluster,
    /// Mode D: Distributed Field Swarm (Massive concurrency simulation).
    FieldSwarm,
}

/// Audit scorecard reporting performance, correctness, and ecology metrics.
#[derive(Debug, Clone, Default)]
pub struct AuditScorecard {
    /// Performance metrics (Latency, Throughput).
    pub performance: PerformanceMetrics,
    /// Correctness metrics (Lawfulness, Violations).
    pub correctness: CorrectnessMetrics,
    /// Ecology metrics (Human Burden, Resource Usage).
    pub ecology: EcologyMetrics,
}

/// Performance metrics for the tournament run.
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total duration of the tournament in milliseconds.
    pub total_duration_ms: u64,
    /// Throughput measured in decisions per second.
    pub throughput_decisions_sec: f64,
}

/// Correctness metrics for the tournament run.
#[derive(Debug, Clone, Default)]
pub struct CorrectnessMetrics {
    /// Total number of decisions made during the tournament.
    pub total_decisions: u64,
    /// Number of decisions that passed lawfulness checks.
    pub lawful_decisions: u64,
    /// Number of decisions that violated process rules.
    pub violations: u64,
}

/// Ecology metrics for the tournament run.
#[derive(Debug, Clone, Default)]
pub struct EcologyMetrics {
    /// Total human burden accumulated during the tournament.
    pub total_human_burden: u64,
    /// Resource footprint (measured in TruthBlock units/octets).
    pub resource_footprint: u64,
}

/// High-concurrency tournament orchestrator.
pub struct MaximumConcurrencyTournament<E, C, CF> {
    /// Current tournament execution mode.
    pub mode: TournamentMode,
    /// Set of environments (one per thread/worker in parallel modes).
    pub environments: Vec<E>,
    /// Shared critic for lawfulness evaluation.
    pub critic: Arc<C>,
    /// Shared counterfactual mutator for stress testing.
    pub counterfactual: Arc<CF>,
    /// Scenarios to be executed across the tournament.
    pub scenarios: Vec<ScenarioFamily>,
}

impl<E, C, CF> MaximumConcurrencyTournament<E, C, CF>
where
    E: CcogEnvironment + Send + 'static,
    C: CcogCritic + Send + Sync + 'static,
    CF: CcogCounterfactual + Send + Sync + 'static,
{
    /// Create a new Maximum Concurrency Tournament.
    pub fn new(
        mode: TournamentMode,
        environments: Vec<E>,
        critic: C,
        counterfactual: CF,
        scenarios: Vec<ScenarioFamily>,
    ) -> Self {
        Self {
            mode,
            environments,
            critic: Arc::new(critic),
            counterfactual: Arc::new(counterfactual),
            scenarios,
        }
    }

    /// Run the tournament and produce an `AuditScorecard`.
    pub fn execute(mut self) -> Result<AuditScorecard> {
        let start = Instant::now();
        let mut scorecard = AuditScorecard::default();

        match self.mode {
            TournamentMode::SingleCore => {
                self.run_sequential(&mut scorecard)?;
            }
            _ => {
                scorecard = self.run_parallel()?;
            }
        }

        scorecard.performance.total_duration_ms = start.elapsed().as_millis() as u64;
        if scorecard.performance.total_duration_ms > 0 {
            scorecard.performance.throughput_decisions_sec = (scorecard.correctness.total_decisions
                as f64)
                / (scorecard.performance.total_duration_ms as f64 / 1000.0);
        }

        Ok(scorecard)
    }

    fn run_sequential(&mut self, scorecard: &mut AuditScorecard) -> Result<()> {
        if self.environments.is_empty() {
            return Ok(());
        }
        let env = &mut self.environments[0];

        for scenario in &self.scenarios {
            env.setup(*scenario)?;

            // Run up to 10 steps per scenario or until Settle/Refuse
            for _ in 0..10 {
                let mut ctx = env.context();
                self.counterfactual.mutate(&mut ctx)?;

                let instinct = crate::instinct::select_instinct_v0(&ctx);
                let decision = crate::runtime::cog8::Cog8Decision {
                    response: instinct.into(),
                    ..Default::default()
                };

                let proof = crate::powl64::Powl64::default();
                let critique_res = self.critic.critique(&ctx, &decision, &proof);

                scorecard.correctness.total_decisions += 1;
                if critique_res.is_ok() {
                    scorecard.correctness.lawful_decisions += 1;
                } else {
                    scorecard.correctness.violations += 1;
                }

                scorecard.ecology.total_human_burden += ctx.human_burden;
                // TruthBlock footprint estimation (1 TruthBlock ~ 1024 octets)
                scorecard.ecology.resource_footprint += 1024;

                env.step(&decision)?;

                if decision.response == Instinct::Settle || decision.response == Instinct::Refuse {
                    break;
                }
            }
        }
        Ok(())
    }

    fn run_parallel(self) -> Result<AuditScorecard> {
        let scenarios = Arc::new(self.scenarios);
        let mut handles = Vec::new();

        // Mode scaling factor
        let threads = match self.mode {
            TournamentMode::SingleCore => 1,
            TournamentMode::MultiCore => self.environments.len(),
            TournamentMode::Cluster => self.environments.len(),
            TournamentMode::FieldSwarm => self.environments.len(),
        };

        for mut env in self.environments.into_iter().take(threads) {
            let scenarios = Arc::clone(&scenarios);
            let critic = Arc::clone(&self.critic);
            let counterfactual = Arc::clone(&self.counterfactual);

            let handle = thread::spawn(move || -> Result<AuditScorecard> {
                let mut local_scorecard = AuditScorecard::default();
                for scenario in scenarios.iter() {
                    env.setup(*scenario)?;

                    for _ in 0..10 {
                        let mut ctx = env.context();
                        counterfactual.mutate(&mut ctx)?;

                        let instinct = crate::instinct::select_instinct_v0(&ctx);
                        let decision = crate::runtime::cog8::Cog8Decision {
                            response: instinct.into(),
                            ..Default::default()
                        };

                        let proof = crate::powl64::Powl64::default();
                        let critique_res = critic.critique(&ctx, &decision, &proof);

                        local_scorecard.correctness.total_decisions += 1;
                        if critique_res.is_ok() {
                            local_scorecard.correctness.lawful_decisions += 1;
                        } else {
                            local_scorecard.correctness.violations += 1;
                        }

                        local_scorecard.ecology.total_human_burden += local_scorecard
                            .ecology
                            .total_human_burden
                            .saturating_add(ctx.human_burden);
                        local_scorecard.ecology.resource_footprint += 1024; // TruthBlock usage

                        env.step(&decision)?;

                        if decision.response == Instinct::Settle
                            || decision.response == Instinct::Refuse
                        {
                            break;
                        }
                    }
                }
                Ok(local_scorecard)
            });
            handles.push(handle);
        }

        let mut final_scorecard = AuditScorecard::default();
        for handle in handles {
            let res = handle
                .join()
                .map_err(|_| anyhow::anyhow!("Tournament thread panicked"))??;
            final_scorecard.correctness.total_decisions += res.correctness.total_decisions;
            final_scorecard.correctness.lawful_decisions += res.correctness.lawful_decisions;
            final_scorecard.correctness.violations += res.correctness.violations;
            final_scorecard.ecology.total_human_burden += res.ecology.total_human_burden;
            final_scorecard.ecology.resource_footprint += res.ecology.resource_footprint;
        }

        Ok(final_scorecard)
    }
}
