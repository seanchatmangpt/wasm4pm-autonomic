use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

<<<<<<< HEAD
<<<<<<< HEAD
pub struct ReinforceAgent<S: WorkflowState, A: WorkflowAction> {
    pub(crate) theta: RefCell<PackedKeyTable<S, QArray>>,
=======
pub struct ReinforceAgent<S: WorkflowState, A: WorkflowAction, V: QValueStore = Vec<f32>> {
    pub(crate) theta: RefCell<PackedKeyTable<S, V>>,
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
=======
pub struct ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    pub(crate) theta: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) _phantom: PhantomData<A>,
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> ReinforceAgent<S, A, V> {
=======
impl<S, A> ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            theta: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: REINFORCE_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            theta: RefCell::new(PackedKeyTable::with_capacity(cap)),
            learning_rate: REINFORCE_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            theta: RefCell::new(StaticPackedKeyTable::new()),
            learning_rate: lr,
            discount_factor: df,
            rng: RefCell::new(Rng::with_seed(seed)),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn with_hyperparams(lr: f32, df: f32) -> Self {
        Self {
            learning_rate: lr,
            discount_factor: df,
            ..Self::new()
        }
    }

    #[allow(dead_code)]
    pub fn select_action(&self, state: S) -> A {
        let theta = self.theta.borrow();
<<<<<<< HEAD
        let weights = get_q_values::<S, A, V>(&*theta, &state);

<<<<<<< HEAD
        let probs = softmax_probs::<ACTION_MAX_LIMIT>(weights);
=======
        let mut admissible_logits = [0.0f32; 64];
        let mut found = false;
        let count = A::ACTION_COUNT.min(64);
        
        for i in 0..count {
            if let Some(a) = A::from_index(i) {
                if state.is_admissible(a) {
                    admissible_logits[i] = weights[i];
                    found = true;
                } else {
                    admissible_logits[i] = -1e9;
                }
            }
        }

        if !found {
            return A::from_index(0).unwrap();
        }

        let probs = softmax_probs(&admissible_logits[..count]);
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
        let u = self.rng.borrow_mut().f32();
        let mut acc = 0.0;

        for (idx, p) in probs.iter().enumerate() {
<<<<<<< HEAD
            if idx >= A::ACTION_COUNT {
                break;
            }
            acc += *p;
            if u <= acc {
                return A::from_index(idx).unwrap();
=======
            if let Some(a) = A::from_index(idx) {
                if state.is_admissible(a) {
                    acc += *p;
                    if u <= acc {
                        return a;
                    }
                }
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
=======
        let h = hash_state(&state);
        let weights = theta.get(h).map(|v| v.as_slice()).unwrap_or(&[0.0; 3][..A::ACTION_COUNT]);

        let max_logit = weights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sum_exp = 0.0;
        let mut exps = [0.0; 3];
        for (i, &w) in weights.iter().enumerate().take(3) {
            let e = (w - max_logit).exp();
            exps[i] = e;
            sum_exp += e;
        }

        let u = self.rng.borrow_mut().f32() * sum_exp;
        let mut acc = 0.0;

        for (i, &e) in exps.iter().enumerate().take(3) {
            acc += e;
            if u <= acc {
                return A::from_index(i).unwrap();
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
            }
        }

        // Fallback to last admissible action
        for i in (0..count).rev() {
            if let Some(a) = A::from_index(i) {
                if state.is_admissible(a) {
                    return a;
                }
            }
        }
        A::from_index(0).unwrap()
    }

    #[allow(dead_code)]
    pub fn update_from_trajectory(&self, trajectory: &[(S, A, f32)]) {
        let n = trajectory.len();
        if n == 0 {
            return;
        }

<<<<<<< HEAD
        // We still need a Vec for returns because trajectory length is dynamic
        // but this is called once per episode, not in the nanosecond-hot loop of select_action.
        let mut returns = vec![0.0f32; n];
=======
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        let mut g = 0.0f32;
        let mut theta = self.theta.borrow_mut();

<<<<<<< HEAD
        for (t, (state, action, _)) in trajectory.iter().enumerate() {
<<<<<<< HEAD
            ensure_state::<S, A>(&mut *theta, *state);
<<<<<<< HEAD
            let logits = get_q_values::<S, A>(&*theta, state);
            let probs = softmax_probs::<ACTION_MAX_LIMIT>(logits);
=======
            let weights = get_q_values::<S, A>(&*theta, state);

            let mut admissible_logits = [0.0f32; 64];
            let count = A::ACTION_COUNT.min(64);
            for i in 0..count {
                if let Some(a) = A::from_index(i) {
                    if state.is_admissible(a) {
                        admissible_logits[i] = weights[i];
                    } else {
                        admissible_logits[i] = -1e9;
                    }
                }
            }
            let probs = softmax_probs(&admissible_logits[..count]);
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
=======
            ensure_state::<S, A, V>(&mut *theta, *state);
            let logits = get_q_values::<S, A, V>(&*theta, state);
            let probs = softmax_probs(logits);
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
            let a_idx = action.to_index();
            let g_t = returns[t];

            let h = hash_state(state);
<<<<<<< HEAD
            let target_weights = theta.get_mut(h).unwrap();
            for j in 0..count {
                if let Some(a) = A::from_index(j) {
                    if state.is_admissible(a) {
                        let grad = if j == a_idx {
                            1.0 - probs[j]
                        } else {
                            -probs[j]
                        };
                        target_weights[j] += self.learning_rate * g_t * grad;
                    }
                }
=======
            let weights = theta.get_mut(h).unwrap().as_mut_slice();
            for j in 0..A::ACTION_COUNT {
                let grad = if j == a_idx {
                    1.0 - probs[j]
                } else {
                    -probs[j]
                };
                weights[j] += self.learning_rate * g_t * grad;
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
=======
        for (_t, (state, action, reward)) in trajectory.iter().enumerate().rev() {
            g = *reward + self.discount_factor * g;

            let h = hash_state(state);
            if theta.get(h).is_none() {
                let _ = theta.insert(h, *state, A::Values::default());
            }
            let weights = theta.get_mut(h).unwrap();

            // Softmax gradient
            let mut sum_exp = 0.0;
            let mut exps = [0.0; 3];
            let w_slice = weights.as_slice();
            let max_logit = w_slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

            for (i, &w) in w_slice.iter().enumerate().take(3) {
                let e = (w - max_logit).exp();
                exps[i] = e;
                sum_exp += e;
            }

            let a_idx = action.to_index();
            for j in 0..A::ACTION_COUNT {
                let p_j = exps[j] / sum_exp;
                let grad = if j == a_idx { 1.0 - p_j } else { -p_j };
                let current = weights.get(j);
                weights.set(j, current + self.learning_rate * g * grad);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
            }
        }
    }

    #[allow(dead_code)]
    pub fn update_step(&self, state: S, action: A, reward: f32) {
        self.update_from_trajectory(&[(state, action, reward)]);
    }

    #[allow(dead_code)]
    pub fn get_policy_weights(&self, state: S) -> A::Values {
        let theta = self.theta.borrow();
<<<<<<< HEAD
        get_q_values::<S, A, V>(&*theta, &state).to_vec()
=======
        *theta.get(hash_state(&state)).unwrap_or(&A::Values::default())
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    }

    pub fn set_exploration_rate(&mut self, _rate: f32) {
        // No-op: REINFORCE uses stochastic policy directly.
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Default for ReinforceAgent<S, A, V> {
=======
impl<S, A> Default for ReinforceAgent<S, A>
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

// Serialization support for ReinforceAgent
impl ReinforceAgent<crate::RlState<1>, crate::RlAction, Vec<f32>> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let theta = self.theta.borrow();
        let mut state_values = std::collections::HashMap::new();

        for (_, state, weights) in theta.iter() {
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
<<<<<<< HEAD
            state_values.insert(key, weights.to_vec());
=======
            state_values.insert(key, weights.as_slice().to_vec());
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
=======
            state_values.insert(key, weights.as_slice().to_vec());
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

        let mut theta = self.theta.borrow_mut();
        theta.clear();

        for (key, weights_vec) in table.state_values {
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
            q_array.copy_from_slice(&weights);
            theta.insert(hash_state(&state), state, q_array);
=======
            let mut weights = [0.0; 3];
            for (i, &w) in weights_vec.iter().enumerate().take(3) {
                weights[i] = w;
            }
            let _ = theta.insert(hash_state(&state), state, weights);
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        }
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> Agent<S, A> for ReinforceAgent<S, A, V> {
=======
impl<S, A> Agent<S, A> for ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&mut self, state: S, action: A, reward: f32, _next_state: S, _done: bool) {
        self.update_step(state, action, reward);
    }

    fn reset(&mut self) {
        let mut theta = self.theta.borrow_mut();
        theta.clear();
    }
}

<<<<<<< HEAD
impl<S: WorkflowState, A: WorkflowAction, V: QValueStore> AgentMeta for ReinforceAgent<S, A, V> {
=======
impl<S, A> AgentMeta for ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
    fn name(&self) -> &'static str {
        "REINFORCE"
    }

    fn exploration_rate(&self) -> f32 {
        0.0
    }

    fn decay_exploration(&mut self) {
        // no-op
    }
}
