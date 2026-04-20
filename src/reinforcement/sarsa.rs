use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// SARSA agent: model-free, on-policy
///
/// This implementation keeps a pending `(next_state, next_action)` pair captured
/// at action-selection time so that the subsequent update can use the actual
/// on-policy next action.
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, Vec<f32>>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) exploration_rate: f32,
    pub(crate) exploration_decay: f32,
    pub(crate) pending_next: RefCell<Option<(S, A)>>,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> SARSAAgent<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(PackedKeyTable::default()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            pending_next: RefCell::new(None),
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            q_table: RefCell::new(PackedKeyTable::default()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            pending_next: RefCell::new(None),
            rng: RefCell::new(Rng::with_seed(seed)),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
        let mut pending = self.pending_next.borrow_mut();
        if let Some((ref s, ref a)) = *pending {
            if *s == state {
                return *a;
            }
        }
        let action = self.epsilon_greedy_action(state, self.exploration_rate);
        *pending = Some((state, action));
        action
    }

    #[allow(dead_code)]
    pub fn epsilon_greedy_action(&self, state: S, epsilon: f32) -> A {
        let eps = clamp_probability(epsilon);
        if self.rng.borrow_mut().f32() < eps {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.greedy_action(state)
        }
    }

    fn greedy_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
        let q_vals = get_q_values::<S, A>(&*q_table, &state);
        A::from_index(greedy_index(q_vals)).unwrap()
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
        ensure_state::<S, A>(&mut *q_table, state);

        let next_q = if done {
            0.0
        } else {
            get_q_values::<S, A>(&*q_table, &next_state)[next_action.to_index()]
        };

        let action_idx = action.to_index();
        let h = hash_state(&state);
        let current_q = q_table.get(h).unwrap()[action_idx];
        let target = reward + self.discount_factor * next_q;
        q_table.get_mut(h).unwrap()[action_idx] += self.learning_rate * (target - current_q);
    }

    #[allow(dead_code)]
    pub fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = clamp_probability(rate);
    }

    #[allow(dead_code)]
    pub fn clear_pending(&self) {
        *self.pending_next.borrow_mut() = None;
    }

    #[allow(dead_code)]
    pub fn get_exploration_rate(&self) -> f32 {
        self.exploration_rate
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
            state_values.insert(key, q_values.clone());
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
            q_table.insert(hash_state(&state), state, q_values);
        }
    }
}

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for SARSAAgent<S, A> {
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        if done {
            self.update_with_next_action(state, action, reward, next_state, action, true);
            return;
        }

        let mut pending = self.pending_next.borrow_mut();
        let next_action = match pending.take() {
            Some((pending_state, pending_action)) if pending_state == next_state => pending_action,
            _ => self.epsilon_greedy_action(next_state, self.exploration_rate),
        };
        // Re-store the next_action so the subsequent select_action uses it
        *pending = Some((next_state, next_action));
        drop(pending);

        self.update_with_next_action(state, action, reward, next_state, next_action, false);
    }

    fn reset(&mut self) {
        self.clear_pending();
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
