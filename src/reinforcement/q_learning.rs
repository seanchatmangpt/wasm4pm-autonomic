use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// Q-Learning agent: model-free, off-policy
pub struct QLearning<S: WorkflowState, A: WorkflowAction>
where
    S: Copy + Default,
    A::Values: Copy + Default,
{
    pub(crate) q_table: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) exploration_rate: f32,
    pub(crate) exploration_decay: f32,
    pub(crate) episodes: RefCell<usize>,
    pub(crate) total_reward: RefCell<f32>,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) _phantom: PhantomData<A>,
}

impl<S, A> QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            episodes: RefCell::new(0),
            total_reward: RefCell::new(0.0),
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            q_table: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            episodes: RefCell::new(0),
            total_reward: RefCell::new(0.0),
            rng: RefCell::new(Rng::with_seed(seed)),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn with_hyperparams(lr: f32, df: f32, exp_rate: f32) -> Self {
        Self {
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: clamp_probability(exp_rate),
            ..Self::new()
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
        if self.rng.borrow_mut().f32() < self.exploration_rate {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.best_action(state)
        }
    }

    fn best_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
        let h = hash_state(&state);
        let q_values = q_table
            .get(h)
            .map(|v| v.as_slice())
            .unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
        A::from_index(greedy_index(q_values)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update(&self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        let mut q_table = self.q_table.borrow_mut();
        let h_state = hash_state(&state);

        if q_table.get(h_state).is_none() {
            let _ = q_table.insert(h_state, state, A::Values::default());
        }

        let next_val = if done {
            0.0
        } else {
            let h_next = hash_state(&next_state);
            q_table
                .get(h_next)
                .map(|v| v.as_slice().iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)))
                .unwrap_or(0.0)
        };

        let action_idx = action.to_index();
        let q_entry = q_table.get_mut(h_state).unwrap();
        let current_q = q_entry.get(action_idx);
        let target = reward + self.discount_factor * next_val;
        q_entry.set(action_idx, current_q + self.learning_rate * (target - current_q));

        *self.total_reward.borrow_mut() += reward;
    }

    #[allow(dead_code)]
    pub fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = clamp_probability(rate);
    }

    #[allow(dead_code)]
    pub fn get_q_value(&self, state: &S, action: &A) -> f32 {
        let q_table = self.q_table.borrow();
        q_table
            .get(hash_state(state))
            .map(|q_vals| q_vals.get(action.to_index()))
            .unwrap_or(0.0)
    }

    #[allow(dead_code)]
    pub fn episode_count(&self) -> usize {
        *self.episodes.borrow()
    }

    #[allow(dead_code)]
    pub fn total_reward(&self) -> f32 {
        *self.total_reward.borrow()
    }

    #[allow(dead_code)]
    pub fn get_exploration_rate(&self) -> f32 {
        self.exploration_rate
    }
}

impl<S, A> Default for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for QLearning
impl QLearning<crate::RlState, crate::RlAction> {
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
            state_values.insert(key, q_values.as_slice().to_vec());
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
            let mut vals = [0.0; 3];
            for (i, &v) in q_values.iter().enumerate().take(3) {
                vals[i] = v;
            }
            let _ = q_table.insert(hash_state(&state), state, vals);
        }
    }
}

impl<S, A> Agent<S, A> for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        self.update(state, action, reward, next_state, done)
    }

    fn reset(&self) {}
}

impl<S, A> AgentMeta for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn name(&self) -> &'static str {
        "QLearning"
    }

    fn exploration_rate(&self) -> f32 {
        self.exploration_rate
    }

    fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }
}
