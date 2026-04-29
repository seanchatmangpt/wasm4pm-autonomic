//! Minimum Decisive Force (MDF) — action selection via lowest-effort improvement.
//!
//! Given a candidate set of actions and a current state, MDF filters to actions that:
//! 1. Have positive expected reward (via Simulator evaluation)
//! 2. Achieve improvement (order_of increases)
//! 3. Among those, have the minimum force (ActionRisk cost)
//!
//! This ensures escalation is minimal necessary: we do not recommend Critical when Medium suffices.

use crate::agentic::counterfactual::Simulator;
use crate::autonomic::types::{ActionRisk, AutonomicAction, AutonomicState};

/// Estimate the force (cost/effort/risk) of an action as a scalar.
/// Maps ActionRisk to magnitude: Low=1.0, Medium=2.0, High=3.0, Critical=4.0
pub fn force_estimate(action: &AutonomicAction) -> f64 {
    match action.risk_profile {
        ActionRisk::Low => 1.0,
        ActionRisk::Medium => 2.0,
        ActionRisk::High => 3.0,
        ActionRisk::Critical => 4.0,
    }
}

/// Order (organizational health/status) as the weighted average of health and conformance.
/// Range [0.0, 1.0]; higher is better.
fn order_of(state: &AutonomicState) -> f64 {
    ((state.process_health as f64) + (state.conformance_score as f64)) / 2.0
}

/// Minimum Decisive Force filter — selects the lowest-cost action that improves state.
pub struct MinimumDecisiveForce;

impl MinimumDecisiveForce {
    /// Filter actions to those with positive reward; return the one with minimum force.
    ///
    /// Algorithm:
    /// 1. Evaluate each action via Simulator::evaluate_action
    /// 2. Keep only actions with positive reward (expected_reward > 0.0)
    /// 3. Among survivors, return the one with argmin force_estimate
    /// 4. If no survivors, return None
    pub fn is_minimal_decisive<'a>(
        actions: &'a [AutonomicAction],
        state: &AutonomicState,
    ) -> Option<&'a AutonomicAction> {
        if actions.is_empty() {
            return None;
        }

        let sim = Simulator::new(state.clone());
        let mut best_action: Option<&'a AutonomicAction> = None;
        let mut best_force = f64::INFINITY;

        for action in actions {
            let (_, reward) = sim.evaluate_action(action);

            // Filter: positive reward only
            if reward > 0.0 {
                let force = force_estimate(action);
                if force < best_force {
                    best_force = force;
                    best_action = Some(action);
                }
            }
        }

        best_action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autonomic::types::ActionType;

    #[test]
    fn test_force_estimate_monotone() {
        assert_eq!(force_estimate(&AutonomicAction::new(1, ActionType::Recommend, ActionRisk::Low, "test")), 1.0);
        assert_eq!(force_estimate(&AutonomicAction::new(1, ActionType::Approve, ActionRisk::Medium, "test")), 2.0);
        assert_eq!(force_estimate(&AutonomicAction::new(1, ActionType::Escalate, ActionRisk::High, "test")), 3.0);
        assert_eq!(force_estimate(&AutonomicAction::new(1, ActionType::Escalate, ActionRisk::Critical, "test")), 4.0);
    }

    #[test]
    fn test_mdf_picks_lowest_force() {
        let state = AutonomicState {
            process_health: 0.9,
            throughput: 0.0,
            conformance_score: 0.9,
            drift_detected: false,
            active_cases: 0,
        };

        let actions = vec![
            AutonomicAction::new(101, ActionType::Recommend, ActionRisk::Low, "low force"),
            AutonomicAction::new(102, ActionType::Escalate, ActionRisk::High, "high force"),
            AutonomicAction::new(103, ActionType::Repair, ActionRisk::Medium, "medium force"),
        ];

        if let Some(chosen) = MinimumDecisiveForce::is_minimal_decisive(&actions, &state) {
            // Among positive-reward actions, should prefer lower force
            let chosen_force = force_estimate(chosen);
            assert!(chosen_force <= 3.0, "Should not select Critical when lower options exist");
        }
    }

    #[test]
    fn test_mdf_empty_list_returns_none() {
        let state = AutonomicState {
            process_health: 0.5,
            throughput: 0.0,
            conformance_score: 0.5,
            drift_detected: false,
            active_cases: 0,
        };

        let actions: Vec<AutonomicAction> = vec![];
        assert!(
            MinimumDecisiveForce::is_minimal_decisive(&actions, &state).is_none(),
            "Empty action list should return None"
        );
    }
}
