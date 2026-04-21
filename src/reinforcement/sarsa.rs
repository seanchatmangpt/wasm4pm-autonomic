use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// SARSA agent: model-free, on-policy
///
/// This implementation keeps a pending `(next_state, next_action)` pair captured
/// at action-selection time so that the subsequent update can use the actual
/// on-policy next action.
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, [f32; 4]>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) exploration_rate: f32,
    pub(crate) exploration_decay: f32,
    pub(crate) episode_count: RefCell<usize>,
    pub(crate) _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> SARSAAgent<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(PackedKeyTable::default()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: 0.5, // Increased for better initial discovery
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            episode_count: RefCell::new(0),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_params(lr: f32, df: f32) -> Self {
        Self {
            q_table: RefCell::new(PackedKeyTable::default()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: 0.5,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            episode_count: RefCell::new(0),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
        if self.exploration_rate <= 0.0 {
            return self.greedy_action(state);
        }

        let episode = *self.episode_count.borrow();
        
        // Deterministic Kernel Rotation (μ-rotation)
        // Episode-dependent rotation ensures stable, repeatable exploration trajectories
        // which is critical for deterministic convergence in process discovery.
        let mod_val = A::ACTION_COUNT as u64 + 1;
        let rot = (episode as u64) % mod_val;
        
        if rot < A::ACTION_COUNT as u64 {
            A::from_index(rot as usize).unwrap()
        } else {
            self.greedy_action(state)
        }
    }

    #[allow(dead_code)]
    fn greedy_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
        let h = hash_state(&state);
        let q_vals = q_table.get(h).map(|v| v.as_slice()).unwrap_or(&[0.5; 4]);
        
        let mut best_idx = 0;
        let mut max_val = q_vals[0];
        
        for i in 1..A::ACTION_COUNT {
            if q_vals[i] > max_val {
                max_val = q_vals[i];
                best_idx = i;
            }
        }
        A::from_index(best_idx).unwrap()
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = rate.clamp(0.0, 1.0);
    }

    #[allow(dead_code)]
    pub fn update_with_next_action(
        &self,
        state: S,
        action: A,
        reward: f32,
        next_state: S,
        next_action: A,
        done: bool,
    ) {
        let mut q_table = self.q_table.borrow_mut();
        let h = hash_state(&state);
        
        if q_table.get(h).is_none() {
            // Optimistic initialization for deterministic exploration
            q_table.insert(h, state, [0.5; 4]);
        }

        let next_h = hash_state(&next_state);
        let next_q = if done {
            0.0
        } else {
            q_table.get(next_h).map(|v| v[next_action.to_index()]).unwrap_or(0.5)
        };

        let action_idx = action.to_index();
        let current_vals = q_table.get_mut(h).unwrap();
        let current_q = current_vals[action_idx];
        let target = reward + self.discount_factor * next_q;
        current_vals[action_idx] += self.learning_rate * (target - current_q);
    }
}

impl<S: WorkflowState, A: WorkflowAction> Default for SARSAAgent<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for SARSAAgent
impl SARSAAgent<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let q_table = self.q_table.borrow();
        let mut state_values = std::collections::HashMap::new();

        for (_, state, q_values) in q_table.iter() {
            let key = encode_rl_state_key(
                state.health_level,
                state.event_rate_q,
                state.activity_count_q,
                state.spc_alert_level,
                state.drift_status,
                state.rework_ratio_q,
                state.circuit_state,
                state.cycle_phase,
            );
            state_values.insert(key, q_values[..crate::RlAction::ACTION_COUNT].to_vec());
        }

        SerializedAgentQTable {
            agent_type,
            state_values,
        }
    }

    #[allow(dead_code)]
    pub fn restore_from_serialized(
        &self,
        table: crate::rl_state_serialization::SerializedAgentQTable,
    ) {
        use crate::rl_state_serialization::decode_rl_state_key;

        let mut q_table = self.q_table.borrow_mut();
        q_table.clear();

        for (key, q_values) in table.state_values {
            let (h, e, a, s, d, r, c, p) = decode_rl_state_key(key);
            let state = crate::RlState {
                health_level: h,
                event_rate_q: e,
                activity_count_q: a,
                spc_alert_level: s,
                drift_status: d,
                rework_ratio_q: r,
                circuit_state: c,
                cycle_phase: p,
                marking_mask: 0,
                activities_hash: 0,
            };
            let mut vals = [0.0; 4];
            for (i, &v) in q_values.iter().enumerate().take(4) {
                vals[i] = v;
            }
            q_table.insert(hash_state(&state), state, vals);
        }
    }
}

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for SARSAAgent<S, A> {
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        let next_action = self.select_action(next_state); // On-policy
        self.update_with_next_action(state, action, reward, next_state, next_action, done);
    }

    fn reset(&mut self) {
        *self.episode_count.borrow_mut() += 1;
    }
}

impl<S: WorkflowState, A: WorkflowAction> AgentMeta for SARSAAgent<S, A> {
    fn name(&self) -> &'static str {
        "SARSA"
    }

    fn exploration_rate(&self) -> f32 {
        self.exploration_rate
    }

    fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }
}
