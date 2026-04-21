use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// Q-Learning agent: model-free, off-policy
<<<<<<< HEAD
<<<<<<< HEAD
pub struct QLearning<S: WorkflowState, A: WorkflowAction> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, QArray>>,
=======
pub struct QLearning<S: WorkflowState, A: WorkflowAction, V: QValueStore = Vec<f32>> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, V>>,
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
=======
pub struct QLearning<S: WorkflowState, A: WorkflowAction>
where
    S: Copy + Default,
    A::Values: Copy + Default,
{
    pub(crate) q_table: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) exploration_rate: f32,
    pub(crate) exploration_decay: f32,
    pub(crate) episodes: RefCell<usize>,
    pub(crate) total_reward: RefCell<f32>,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) deterministic: bool,
    pub(crate) _phantom: PhantomData<A>,
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> QLearning<S, A, V> {
=======
impl<S, A> QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
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
            deterministic: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            q_table: RefCell::new(PackedKeyTable::with_capacity(cap)),
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
            deterministic: false,
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

    pub fn set_deterministic(&mut self, deterministic: bool) {
        self.deterministic = deterministic;
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
<<<<<<< HEAD
        if !self.deterministic && self.rng.borrow_mut().f32() < self.exploration_rate {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
=======
        if self.rng.borrow_mut().f32() < self.exploration_rate {
            // Randomly select an ADMISSIBLE action (Zero-heap)
            let mut count = 0;
            for i in 0..A::ACTION_COUNT {
                if let Some(a) = A::from_index(i) {
                    if state.is_admissible(a) {
                        count += 1;
                    }
                }
            }
            
            if count == 0 {
                return A::from_index(0).unwrap(); // Fallback
            }
            
            let mut choice = self.rng.borrow_mut().usize(..count);
            for i in 0..A::ACTION_COUNT {
                if let Some(a) = A::from_index(i) {
                    if state.is_admissible(a) {
                        if choice == 0 {
                            return a;
                        }
                        choice -= 1;
                    }
                }
            }
            A::from_index(0).unwrap()
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
        } else {
            self.best_action(state)
        }
    }

    fn best_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
<<<<<<< HEAD
<<<<<<< HEAD
        let q_values = get_q_values::<S, A>(&*q_table, &state);
        
        let mut best_idx = 0;
        let mut max_val = f32::NEG_INFINITY;
        let mut found = false;

        for i in 0..A::ACTION_COUNT {
            if let Some(a) = A::from_index(i) {
                if state.is_admissible(a) {
                    if q_values[i] > max_val || !found {
                        max_val = q_values[i];
                        best_idx = i;
                        found = true;
                    }
                }
            }
        }
        A::from_index(best_idx).unwrap()
=======
        let q_values = get_q_values::<S, A, V>(&*q_table, &state);
=======
        let h = hash_state(&state);
        let q_values = q_table
            .get(h)
            .map(|v| v.as_slice())
            .unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        A::from_index(greedy_index(q_values)).unwrap()
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
    }

    #[allow(dead_code)]
    pub fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        let mut q_table = self.q_table.borrow_mut();
<<<<<<< HEAD
        ensure_state::<S, A, V>(&mut *q_table, state);
=======
        let h_state = hash_state(&state);

        if q_table.get(h_state).is_none() {
            let _ = q_table.insert(h_state, state, A::Values::default());
        }
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths

        let next_val = if done {
            0.0
        } else {
<<<<<<< HEAD
            max_q::<S, A, V>(&*q_table, &next_state)
        };

        let action_idx = action.to_index();
        let h = hash_state(&state);
        let current_q = q_table.get(h).unwrap().as_slice()[action_idx];
        let target = reward + self.discount_factor * next_val;
        q_table.get_mut(h).unwrap().as_mut_slice()[action_idx] += self.learning_rate * (target - current_q);
=======
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
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths

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
<<<<<<< HEAD
            .map(|q_vals| q_vals.as_slice()[action.to_index()])
=======
            .map(|q_vals| q_vals.get(action.to_index()))
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
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

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Default for QLearning<S, A, V> {
=======
impl<S, A> Default for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for QLearning
impl QLearning<crate::RlState<1>, crate::RlAction, Vec<f32>> {
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
<<<<<<< HEAD
            // QArray is copyable, so we can directly store it or clone it if needed
            state_values.insert(key, q_values.to_vec());
=======
            state_values.insert(key, q_values.as_slice().to_vec());
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
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
<<<<<<< HEAD
                marking_mask: KBitSet::zero(),
=======
                marking_mask: crate::utils::dense_kernel::K1024::zero(),
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
                activities_hash: 0,
                ontology_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
<<<<<<< HEAD
                universe: None,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
            };
<<<<<<< HEAD
            let mut q_arr = [0.0; ACTION_MAX_LIMIT];
            let len = q_values.len().min(ACTION_MAX_LIMIT);
            q_arr[..len].copy_from_slice(&q_values[..len]);
            q_table.insert(hash_state(&state), state, q_arr);
=======
            let mut vals = [0.0; 3];
            for (i, &v) in q_values.iter().enumerate().take(3) {
                vals[i] = v;
            }
            let _ = q_table.insert(hash_state(&state), state, vals);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        }
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Agent<S, A> for QLearning<S, A, V> {
=======
impl<S, A> Agent<S, A> for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool) {
        self.update(state, action, reward, next_state, done)
    }

    fn reset(&mut self) {}
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> AgentMeta for QLearning<S, A, V> {
=======
impl<S, A> AgentMeta for QLearning<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
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
