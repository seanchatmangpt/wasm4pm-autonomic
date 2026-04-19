//! Counterfactual simulation engine for evaluating proposed actions before commitment.
//! Implements a safe "What-If" shadow state.

use crate::autonomic::types::{AutonomicAction, AutonomicState, ActionType};
use crate::utils::dense_kernel::K64;
use crate::utils::bitset::select_u64;

pub struct Simulator {
    baseline_state: AutonomicState,
}

impl Simulator {
    pub fn new(state: AutonomicState) -> Self {
        Self { baseline_state: state }
    }

    /// Evaluates the projected state if an action were to be applied.
    /// Uses BCINR select_u64 for branchless state mutation logic.
    pub fn evaluate_action(&self, action: &AutonomicAction) -> (AutonomicState, f32) {
        let mut projected = self.baseline_state.clone();
        
        let is_repair = (action.action_type == ActionType::Repair) as u64;
        let is_recommend = (action.action_type == ActionType::Recommend) as u64;
        
        // Use select_u64 for boolean-like drift reset
        projected.drift_detected = select_u64(is_repair, 0, projected.drift_detected as u64) != 0;
        
        // Branchless reward selection (mapped to discrete values)
        // High = 0.8 (Repair), Med = 0.2 (Recommend), Low = 0.05 (Escalate)
        let mut reward = 0.05; // Default Escalate/Other
        reward = if is_repair != 0 { 0.8 } else if is_recommend != 0 { 0.2 } else { reward };

        (projected, reward)
    }

    /// Tests a sequence of actions (a policy option) and returns the cumulative expected reward.
    pub fn rollout_policy(&self, actions: &[AutonomicAction]) -> f32 {
        let mut current_state = self.baseline_state.clone();
        let mut cumulative_reward = 0.0;

        for action in actions {
            let sim = Simulator::new(current_state);
            let (next_state, reward) = sim.evaluate_action(action);
            current_state = next_state;
            cumulative_reward += reward;
        }

        cumulative_reward
    }
}
