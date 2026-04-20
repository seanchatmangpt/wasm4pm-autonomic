use crate::autonomic::types::*;
use crate::config::AutonomicConfig;
use log::{debug, info, warn};

pub trait AutonomicKernel {
    fn observe(&mut self, event: AutonomicEvent);
    fn infer(&self) -> AutonomicState;
    fn propose(&self, state: &AutonomicState) -> Vec<AutonomicAction>;
    fn accept(&self, action: &AutonomicAction, state: &AutonomicState) -> bool;
    fn execute(&mut self, action: AutonomicAction) -> AutonomicResult;
    fn manifest(&self, result: &AutonomicResult) -> String;
    fn adapt(&mut self, feedback: AutonomicFeedback);

    /// High-level helper to run a full autonomic cycle for a given event.
    /// This is the "Digital Team" entry point.
    ///
    /// NOTE TO AGENT: The 'propose' and 'accept' phases are ideal for parallel
    /// orchestration using Gemini CLI subagents. Consider spawning parallel
    /// rollouts for complex action simulations.
    fn run_cycle(&mut self, event: AutonomicEvent) -> Vec<AutonomicResult> {
        info!("Starting autonomic cycle for event from {}", event.source);
        self.observe(event);
        let state = self.infer();
        debug!("Inferred state: {:?}", state);

        // Only act if health and conformance are above safety thresholds.
        // If conformance < 0.75, an autonomous agent should pause and run 'Deep Diagnostics'.
        let config = AutonomicConfig::load("dteam.toml").unwrap_or_default();
        if state.process_health < config.autonomic.guards.min_health_threshold
            || state.conformance_score < 0.75
        {
            warn!(
                "Safety threshold breached: health={}, conformance={}. Pausing cycle.",
                state.process_health, state.conformance_score
            );
            return Vec::new();
        }

        let actions = self.propose(&state);
        info!("Proposed {} actions", actions.len());

        let mut results = Vec::new();
        for action in actions {
            if self.accept(&action, &state) {
                info!("Executing accepted action: {}", action.parameters);
                let result = self.execute(action);
                results.push(result);
            } else {
                debug!("Action rejected: {}", action.parameters);
            }
        }
        info!("Autonomic cycle complete. {} actions executed.", results.len());
        results
    }
}

pub struct DefaultKernel {
    pub last_event: Option<AutonomicEvent>,
    pub state: AutonomicState,
    pub config: AutonomicConfig,
}

impl Default for DefaultKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultKernel {
    pub fn new() -> Self {
        let config = AutonomicConfig::load("dteam.toml").unwrap_or_default();
        Self {
            last_event: None,
            state: AutonomicState {
                process_health: 1.0,
                throughput: 0.0,
                conformance_score: 1.0,
                drift_detected: false,
                active_cases: 0,
            },
            config,
        }
    }
}

impl AutonomicKernel for DefaultKernel {
    fn observe(&mut self, event: AutonomicEvent) {
        debug!("Observing event from {}: {}", event.source, event.payload);
        self.last_event = Some(event);
    }

    fn infer(&self) -> AutonomicState {
        self.state.clone()
    }

    fn propose(&self, _state: &AutonomicState) -> Vec<AutonomicAction> {
        let mode = &self.config.autonomic.mode;
        debug!("Proposing actions in '{}' mode", mode);

        if mode == "recommend" {
            return vec![AutonomicAction::recommend(1, "Optimize flow")];
        }

        vec![
            AutonomicAction::recommend(1, "Optimize flow"),
            AutonomicAction::new(
                2,
                ActionType::Repair,
                ActionRisk::Medium,
                "Repair local drift",
            ),
            AutonomicAction::new(
                3,
                ActionType::Repair,
                ActionRisk::High,
                "Global structural re-alignment (Dr. Wil Special)",
            ),
        ]
    }

    fn accept(&self, action: &AutonomicAction, _state: &AutonomicState) -> bool {
        // van der Aalst Soundness Guard
        // If strict_conformance is on, we reject any action that could jeopardize structural soundness
        if self.config.autonomic.policy.profile == "strict_conformance" {
            debug!("Strict conformance policy active. Verifying action: {}", action.parameters);
            // For structural repair actions, we would normally run a soundness verifier here.
            // For this baseline, we ensure critical risk actions are only accepted if
            // the model satisfies WF-net soundness.
            if action.risk_profile >= ActionRisk::High {
                warn!("Rejecting high-risk action under strict conformance: {}", action.parameters);
                // Mock: In a real implementation, this would call PetriNet::is_structural_workflow_net()
                // on the projected model after applying the action.
                return false;
            }
        }

        // Use risk threshold from config
        let threshold = match self.config.autonomic.guards.risk_threshold.as_str() {
            "Low" => ActionRisk::Low,
            "Medium" => ActionRisk::Medium,
            "High" => ActionRisk::High,
            _ => ActionRisk::Critical,
        };

        let accepted = action.risk_profile <= threshold;
        if !accepted {
            warn!("Action rejected due to risk threshold: risk={:?}, threshold={:?}", action.risk_profile, threshold);
        }
        accepted
    }

    fn execute(&mut self, action: AutonomicAction) -> AutonomicResult {
        debug!("Executing action ID {}: {}", action.action_id, action.parameters);
        AutonomicResult {
            success: true,
            execution_latency_ms: 10,
            manifest_hash: 0xDEADBEEF,
        }
    }

    fn manifest(&self, result: &AutonomicResult) -> String {
        format!(
            "MANIFEST: success={}, hash={:X} [Integrity: {}]",
            result.success, result.manifest_hash, self.config.autonomic.integrity_hash
        )
    }

    fn adapt(&mut self, feedback: AutonomicFeedback) {
        info!("Adapting system based on feedback (reward={})", feedback.reward);
        let old_health = self.state.process_health;
        if feedback.reward > 0.0 {
            self.state.process_health =
                (self.state.process_health + feedback.reward * 0.01).min(1.0);
        } else {
            // Negative reward reduces health
            self.state.process_health =
                (self.state.process_health + feedback.reward * 0.1).max(0.0);
        }

        debug!("Health updated: {} -> {}", old_health, self.state.process_health);

        if feedback.human_override {
            warn!("Human override detected. Flagging drift.");
            self.state.drift_detected = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_autonomic_lifecycle() {
        let mut kernel = DefaultKernel::new();

        // 1. Observe
        kernel.observe(AutonomicEvent {
            source: "sensor_1".to_string(),
            payload: "event_data".to_string(),
            timestamp: SystemTime::now(),
        });

        // 2. Infer
        let state = kernel.infer();
        assert!(state.process_health > 0.0);

        // 3. Propose
        let actions = kernel.propose(&state);
        assert!(!actions.is_empty());

        // 4. Accept
        let action = actions[0].clone();
        let accepted = kernel.accept(&action, &state);
        assert!(accepted);

        // 5. Execute
        let result = kernel.execute(action);
        assert!(result.success);

        // 6. Manifest
        let manifest = kernel.manifest(&result);
        assert!(manifest.contains("DEADBEEF"));

        // 7. Adapt
        kernel.adapt(AutonomicFeedback {
            reward: 1.0,
            human_override: false,
            side_effects: vec![],
        });
    }
}
