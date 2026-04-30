use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// SARSA agent: model-free, on-policy
///
/// This implementation keeps a pending `(next_state, next_action)` pair captured
/// at action-selection time so that the subsequent update can use the actual
/// on-policy next action.
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, QArray>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
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
            episode_count: RefCell::new(0),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
        // Deterministic rotation of actions for exploration:
        // Every 3 episodes, we take a different exploratory action,
        // otherwise we are greedy.
        let episode = *self.episode_count.borrow();
        if episode % 3 == 1 {
            // Exploratory action 1
            A::from_index(0)
                .expect("valid action index — out-of-bounds is a caller contract violation")
        } else if episode % 3 == 2 {
            // Exploratory action 2
            A::from_index(1)
                .expect("valid action index — out-of-bounds is a caller contract violation")
        } else {
            // Greedy
            self.greedy_action(state)
        }
    }

    #[allow(dead_code)]
    fn greedy_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
        let q_vals = get_q_values::<S, A>(&*q_table, &state);
        let idx = q_vals
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        A::from_index(idx)
            .expect("valid action index — out-of-bounds is a caller contract violation")
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
        ensure_state::<S>(&mut *q_table, state);

        let next_q = if done {
            0.0
        } else {
            get_q_values::<S, A>(&*q_table, &next_state)[next_action.to_index()]
        };

        let action_idx = action.to_index();
        let h = hash_state(&state);
        let current_q = q_table
            .get_mut(h)
            .expect("state previously ensured to exist")[action_idx];
        let target = reward + self.discount_factor * next_q;
        q_table
            .get_mut(h)
            .expect("state previously ensured to exist")[action_idx] +=
            self.learning_rate * (target - current_q);
    }
}

impl<S: WorkflowState, A: WorkflowAction> Default for SARSAAgent<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for SARSAAgent
impl SARSAAgent<crate::RlState<1>, crate::RlAction> {
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
            state_values.insert(key, q_values.to_vec());
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
        use crate::utils::dense_kernel::KBitSet;

        let mut q_table = self.q_table.borrow_mut();
        q_table.clear();

        for (key, q_values) in table.state_values {
            let (h, e, a, s, d, r, c, p) = decode_rl_state_key(key);
            let state = crate::RlState::<1> {
                health_level: h,
                event_rate_q: e,
                activity_count_q: a,
                spc_alert_level: s,
                drift_status: d,
                rework_ratio_q: r,
                circuit_state: c,
                cycle_phase: p,
                marking_mask: KBitSet::zero(),
                activities_hash: 0,
                ontology_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
                universe: None,
            };
            let mut q_array = [0.0; ACTION_MAX_LIMIT];
            q_array.copy_from_slice(&q_values);
            q_table.insert(hash_state(&state), state, q_array);
        }
    }
}

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for SARSAAgent<S, A> {
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        let next_action = self.greedy_action(next_state);
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
        0.0
    }

    fn decay_exploration(&mut self) {}
}
