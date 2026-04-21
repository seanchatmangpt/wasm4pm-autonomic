//! Counterfactual simulation engine for evaluating proposed actions before commitment.
//! Implements a safe "What-If" shadow state.
use crate::autonomic::types::{ActionType, AutonomicResult, AutonomicState};
use crate::utils::bitset::select_u64;

const REWARD_REPAIR: f32 = 0.8;
const REWARD_RECOMMEND: f32 = 0.2;
const REWARD_ESCALATE: f32 = 0.05;

pub struct Simulator {
    baseline_state: AutonomicState,
}

impl Simulator {
    pub fn new(state: AutonomicState) -> Self {
        Self {
            baseline_state: state,
        }
    }

    /// Evaluates the projected state if an action were to be applied.
    /// Uses BCINR select_u64 for branchless state mutation logic.
    pub fn evaluate_action(&self, action_idx: usize) -> (AutonomicState, f32) {
        let mut projected = self.baseline_state;

        let is_repair = (action_idx == 2) as u64; // Based on DefaultKernel mapping
        let is_recommend = (action_idx == 0) as u64;

        // Use select_u64 for boolean-like drift reset
        projected.drift_detected = select_u64(is_repair, 0, projected.drift_detected as u64) != 0;

        // Branchless reward selection
        let mut reward = REWARD_ESCALATE;
        if is_repair != 0 {
            reward = REWARD_REPAIR;
        } else if is_recommend != 0 {
            reward = REWARD_RECOMMEND;
        }

        (projected, reward)
    }

    /// Tests a sequence of actions (a policy option) and returns the cumulative expected reward.
    pub fn rollout_policy(&self, actions: &[usize]) -> f32 {
        let mut current_state = self.baseline_state;
        let mut cumulative_reward = 0.0;

        for &action_idx in actions {
            let sim = Simulator::new(current_state);
            let (next_state, reward) = sim.evaluate_action(action_idx);
            current_state = next_state;
            cumulative_reward += reward;
        }

        cumulative_reward
    }
}
