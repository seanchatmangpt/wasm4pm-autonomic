use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

pub struct DoubleQLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    pub(crate) q_a: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
    pub(crate) q_b: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) exploration_rate: f32,
    pub(crate) exploration_decay: f32,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) _phantom: PhantomData<A>,
}

impl<S, A> DoubleQLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_a: RefCell::new(StaticPackedKeyTable::new()),
            q_b: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            q_a: RefCell::new(StaticPackedKeyTable::new()),
            q_b: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
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
            self.greedy_action(state)
        }
    }

    fn greedy_action(&self, state: S) -> A {
        let qa = self.q_a.borrow();
        let qb = self.q_b.borrow();
        let h = hash_state(&state);

        let va = qa.get(h).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
        let vb = qb.get(h).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);

        let mut merged = A::Values::default();
        let m_slice = merged.as_mut_slice();
        for i in 0..A::ACTION_COUNT {
            m_slice[i] = va[i] + vb[i];
        }

        A::from_index(greedy_index(m_slice)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update(&self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        let mut qa = self.q_a.borrow_mut();
        let mut qb = self.q_b.borrow_mut();

        let h_state = hash_state(&state);
        let h_next = hash_state(&next_state);

        if qa.get(h_state).is_none() {
            let _ = qa.insert(h_state, state, A::Values::default());
        }
        if qb.get(h_state).is_none() {
            let _ = qb.insert(h_state, state, A::Values::default());
        }

        let action_idx = action.to_index();

        if self.rng.borrow_mut().bool() {
            let next_q = if done {
                0.0
            } else {
                let next_vals = qa.get(h_next).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
                let best_next_idx = greedy_index(next_vals);
                qb.get(h_next)
                    .map(|vals| vals.get(best_next_idx))
                    .unwrap_or(0.0)
            };

            let q_entry = qa.get_mut(h_state).unwrap();
            let current = q_entry.get(action_idx);
            let target = reward + self.discount_factor * next_q;
            q_entry.set(action_idx, current + self.learning_rate * (target - current));
        } else {
            let next_q = if done {
                0.0
            } else {
                let next_vals = qb.get(h_next).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
                let best_next_idx = greedy_index(next_vals);
                qa.get(h_next)
                    .map(|vals| vals.get(best_next_idx))
                    .unwrap_or(0.0)
            };

            let q_entry = qb.get_mut(h_state).unwrap();
            let current = q_entry.get(action_idx);
            let target = reward + self.discount_factor * next_q;
            q_entry.set(action_idx, current + self.learning_rate * (target - current));
        }
    }

    #[allow(dead_code)]
    pub fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = clamp_probability(rate);
    }

    #[allow(dead_code)]
    pub fn get_exploration_rate(&self) -> f32 {
        self.exploration_rate
    }
}

impl<S, A> Default for DoubleQLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for DoubleQLearning
impl DoubleQLearning<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let qa = self.q_a.borrow();
        let mut state_values = std::collections::HashMap::new();

        for (_, state, q_values) in qa.iter() {
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

        let mut qa = self.q_a.borrow_mut();
        let mut qb = self.q_b.borrow_mut();
        qa.clear();
        qb.clear();

        for (key, q_values_vec) in table.state_values {
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
            let mut q_values = [0.0; 3];
            for (i, &v) in q_values_vec.iter().enumerate().take(3) {
                q_values[i] = v;
            }
            let _ = qa.insert(hash_state(&state), state, q_values);
            let _ = qb.insert(hash_state(&state), state, q_values);
        }
    }
}

impl<S, A> Agent<S, A> for DoubleQLearning<S, A>
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

impl<S, A> AgentMeta for DoubleQLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn name(&self) -> &'static str {
        "DoubleQLearning"
    }

    fn exploration_rate(&self) -> f32 {
        self.exploration_rate
    }

    fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }
}
