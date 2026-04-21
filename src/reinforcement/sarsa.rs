<<<<<<< HEAD
=======
use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

/// SARSA agent: model-free, on-policy

/// This implementation keeps a pending `(next_state, next_action)` pair captured
/// at action-selection time so that the subsequent update can use the actual
/// on-policy next action.
<<<<<<< HEAD
<<<<<<< HEAD
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, QArray>>,
=======
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction, V: QValueStore = Vec<f32>> {
    pub(crate) q_table: RefCell<PackedKeyTable<S, V>>,
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
=======
pub struct SARSAAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    pub(crate) q_table: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) episode_count: RefCell<usize>,
    pub(crate) _phantom: PhantomData<A>,
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> SARSAAgent<S, A, V> {
=======
impl<S, A> SARSAAgent<S, A>
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
            episode_count: RefCell::new(0),
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
            pending_next: RefCell::new(None),
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_params(lr: f32, df: f32) -> Self {
        Self {
            q_table: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: lr,
            discount_factor: df,
            episode_count: RefCell::new(0),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
<<<<<<< HEAD
        // Deterministic rotation of actions for exploration:
        // Every 3 episodes, we take a different exploratory action,
        // otherwise we are greedy.
        let episode = *self.episode_count.borrow();
        if episode % 3 == 1 {
            // Exploratory action 1
            A::from_index(0).unwrap()
        } else if episode % 3 == 2 {
            // Exploratory action 2
            A::from_index(1).unwrap()
=======
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
            // Greedy
            self.greedy_action(state)
        }
    }

    #[allow(dead_code)]
    fn greedy_action(&self, state: S) -> A {
        let q_table = self.q_table.borrow();
<<<<<<< HEAD
<<<<<<< HEAD
        let q_vals = get_q_values::<S, A>(&*q_table, &state);
<<<<<<< HEAD
        let idx = q_vals
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        A::from_index(idx).unwrap()
=======
        
        let mut best_idx = 0;
        let mut max_val = f32::NEG_INFINITY;
        let mut found = false;

        for i in 0..A::ACTION_COUNT {
            if let Some(a) = A::from_index(i) {
                if state.is_admissible(a) {
                    if q_vals[i] > max_val || !found {
                        max_val = q_vals[i];
                        best_idx = i;
                        found = true;
                    }
                }
            }
        }
        A::from_index(best_idx).unwrap()
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
=======
        let q_vals = get_q_values::<S, A, V>(&*q_table, &state);
=======
        let q_vals = q_table.get(hash_state(&state)).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        A::from_index(greedy_index(q_vals)).unwrap()
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
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
<<<<<<< HEAD
        ensure_state::<S, A, V>(&mut *q_table, state);
=======
        let h_state = hash_state(&state);
        if q_table.get(h_state).is_none() {
            let _ = q_table.insert(h_state, state, A::Values::default());
        }
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths

        let next_q = if done {
            0.0
        } else {
<<<<<<< HEAD
            get_q_values::<S, A, V>(&*q_table, &next_state)[next_action.to_index()]
        };

        let action_idx = action.to_index();
        let h = hash_state(&state);
<<<<<<< HEAD
        let current_q = q_table.get_mut(h).unwrap()[action_idx];
=======
        let current_q = q_table.get(h).unwrap().as_slice()[action_idx];
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
        let target = reward + self.discount_factor * next_q;
        q_table.get_mut(h).unwrap().as_mut_slice()[action_idx] += self.learning_rate * (target - current_q);
    }
}

impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Default for SARSAAgent<S, A, V> {
=======
            q_table.get(hash_state(&next_state))
                .map(|v| v.get(next_action.to_index()))
                .unwrap_or(0.0)
        };

        let action_idx = action.to_index();
        let q_entry = q_table.get_mut(h_state).unwrap();
        let current_q = q_entry.get(action_idx);
        let target = reward + self.discount_factor * next_q;
        q_entry.set(action_idx, current_q + self.learning_rate * (target - current_q));
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

impl<S, A> Default for SARSAAgent<S, A>
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

// Serialization support for SARSAAgent
impl SARSAAgent<crate::RlState<1>, crate::RlAction, Vec<f32>> {
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

        for (key, q_values_vec) in table.state_values {
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
            let mut q_array = [0.0; ACTION_MAX_LIMIT];
            q_array.copy_from_slice(&q_values);
            q_table.insert(hash_state(&state), state, q_array);
=======
            let mut q_values = [0.0; 3];
            for (i, &v) in q_values_vec.iter().enumerate().take(3) {
                q_values[i] = v;
            }
            let _ = q_table.insert(hash_state(&state), state, q_values);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        }
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Agent<S, A> for SARSAAgent<S, A, V> {
=======
impl<S, A> Agent<S, A> for SARSAAgent<S, A>
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
        let next_action = self.greedy_action(next_state);
        self.update_with_next_action(state, action, reward, next_state, next_action, done);
    }

    fn reset(&mut self) {
        *self.episode_count.borrow_mut() += 1;
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> AgentMeta for SARSAAgent<S, A, V> {
=======
impl<S, A> AgentMeta for SARSAAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    fn name(&self) -> &'static str {
        "SARSA"
    }

    fn exploration_rate(&self) -> f32 {
        0.0
    }

    fn decay_exploration(&mut self) {}
}
