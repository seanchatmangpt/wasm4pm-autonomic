//! Core traits and utilities for reinforcement learning
//! in single-threaded WASM environments.

pub use crate::utils::dense_kernel::PackedKeyTable;
use std::hash::{Hash, Hasher};

pub const DEFAULT_LEARNING_RATE: f32 = 0.1;
pub const DEFAULT_DISCOUNT_FACTOR: f32 = 0.99;
pub const DEFAULT_EXPLORATION_RATE: f32 = 1.0;
pub const DEFAULT_EXPLORATION_DECAY: f32 = 0.995;
pub const REINFORCE_LEARNING_RATE: f32 = 0.01;

pub mod double_q;
pub mod expected_sarsa;
pub mod q_learning;
pub mod reinforce;
pub mod sarsa;

pub use double_q::DoubleQLearning;
pub use expected_sarsa::ExpectedSARSAAgent;
pub use q_learning::QLearning;
pub use reinforce::ReinforceAgent;
pub use sarsa::SARSAAgent;

<<<<<<< HEAD
/// Trait for storing Q-values, allowing for both heap (Vec) and stack (Array) storage.
pub trait QValueStore: Clone {
    fn new(size: usize) -> Self;
    fn as_slice(&self) -> &[f32];
    fn as_mut_slice(&mut self) -> &mut [f32];
}

impl QValueStore for Vec<f32> {
    #[inline]
    fn new(size: usize) -> Self {
        vec![0.0; size]
    }
    #[inline]
    fn as_slice(&self) -> &[f32] {
        self
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [f32] {
        self
    }
}

/// Stack-allocated Q-values for zero-heap hot paths.
#[derive(Clone, Copy, Debug)]
pub struct StaticQValues<const N: usize> {
    pub values: [f32; N],
}

impl<const N: usize> Default for StaticQValues<N> {
    fn default() -> Self {
        Self {
            values: [0.0; N],
        }
    }
}

impl<const N: usize> QValueStore for StaticQValues<N> {
    #[inline]
    fn new(_size: usize) -> Self {
        Self::default()
    }
    #[inline]
    fn as_slice(&self) -> &[f32] {
        &self.values
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.values
    }
}

/// State for reinforcement learning (must be hashable and copyable)
pub trait WorkflowState: Clone + Copy + Eq + Hash {
<<<<<<< HEAD
    /// State features for function approximation (zero-heap)
    fn features(&self) -> [f32; 16];
=======
    /// Dimension of state features for function approximation
    const FEATURE_DIM: usize;

    /// State features for function approximation (zero-allocation)
    fn write_features(&self, out: &mut [f32]);
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
=======
/// Fixed-size array of action values to eliminate heap allocations.
pub trait ActionArray: Default + Clone + Copy + Send + Sync {
    fn get(&self, index: usize) -> f32;
    fn set(&mut self, index: usize, value: f32);
    fn len(&self) -> usize;
    fn as_slice(&self) -> &[f32];
    fn as_mut_slice(&mut self) -> &mut [f32];
}
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths

impl ActionArray for [f32; 3] {
    #[inline]
    fn get(&self, index: usize) -> f32 {
        self[index]
    }
    #[inline]
    fn set(&mut self, index: usize, value: f32) {
        self[index] = value;
    }
    #[inline]
    fn len(&self) -> usize {
        3
    }
    #[inline]
    fn as_slice(&self) -> &[f32] {
        self
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [f32] {
        self
    }
}

/// State for reinforcement learning (must be hashable and copyable)
pub trait WorkflowState: Clone + Copy + Eq + Hash + Send + Sync {
    /// Is this a terminal state?
    fn is_terminal(&self) -> bool;

    /// Is this action admissible in the current state?
    fn is_admissible<A: WorkflowAction>(&self, _action: A) -> bool {
        true
    }
}

/// Action for reinforcement learning (must be copyable)
pub trait WorkflowAction: Clone + Copy + Eq + Hash + Send + Sync {
    /// Total number of possible actions
    const ACTION_COUNT: usize;

    /// Associated fixed-size array type for action values
    type Values: ActionArray;

    /// Convert to index (0..ACTION_COUNT)
    fn to_index(&self) -> usize;

    /// Convert from index
    fn from_index(idx: usize) -> Option<Self>;
}

/// Trait for any learning agent
pub trait Agent<S: WorkflowState, A: WorkflowAction> {
    fn select_action(&self, state: S) -> A;
    fn update(&mut self, state: S, action: A, reward: f32, next_state: S, done: bool);
    fn reset(&mut self);
}

/// Metadata trait for agent introspection
pub trait AgentMeta {
    fn name(&self) -> &'static str;
    fn exploration_rate(&self) -> f32;
    fn decay_exploration(&mut self);
}

// --- Common Utilities ---

#[inline]
pub(crate) fn clamp_probability(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

#[inline]
pub(crate) fn decay_probability(current: f32, decay: f32) -> f32 {
    clamp_probability(current * decay)
}

pub(crate) fn greedy_index(values: &[f32]) -> usize {
    let mut best_idx = 0;
    let mut max_val = f32::NEG_INFINITY;

    for (idx, &val) in values.iter().enumerate() {
        if val > max_val {
            max_val = val;
            best_idx = idx;
        }
    }
    best_idx
}

#[inline]
pub(crate) fn hash_state<S: Hash>(state: &S) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    state.hash(&mut hasher);
    hasher.finish()
}
<<<<<<< HEAD

<<<<<<< HEAD
pub const ACTION_MAX_LIMIT: usize = 8;
pub type QArray = [f32; ACTION_MAX_LIMIT];

pub(crate) fn get_q_values<'a, S, A>(table: &'a PackedKeyTable<S, QArray>, state: &S) -> &'a [f32]
=======
pub(crate) fn get_q_values<'a, S, A, V>(table: &'a PackedKeyTable<S, V>, state: &S) -> &'a [f32]
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
where
    S: WorkflowState,
    A: WorkflowAction,
    V: QValueStore,
{
    static ZEROS: QArray = [0.0; ACTION_MAX_LIMIT];
    table
        .get(hash_state(state))
        .map(|v| &v[..A::ACTION_COUNT])
        .unwrap_or(&ZEROS[..A::ACTION_COUNT])
}

<<<<<<< HEAD
pub(crate) fn ensure_state<S, A>(table: &mut PackedKeyTable<S, QArray>, state: S)
=======
pub(crate) fn ensure_state<S, A, V>(table: &mut PackedKeyTable<S, V>, state: S)
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
where
    S: WorkflowState,
    A: WorkflowAction,
    V: QValueStore,
{
    let h = hash_state(&state);
    if table.get(h).is_none() {
<<<<<<< HEAD
        table.insert(h, state, [0.0; ACTION_MAX_LIMIT]);
    }
}

pub(crate) fn max_q<S, A>(table: &PackedKeyTable<S, QArray>, state: &S) -> f32
=======
        table.insert(h, state, V::new(A::ACTION_COUNT));
    }
}

pub(crate) fn max_q<S, A, V>(table: &PackedKeyTable<S, V>, state: &S) -> f32
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
where
    S: WorkflowState,
    A: WorkflowAction,
    V: QValueStore,
{
<<<<<<< HEAD
    let q_values = get_q_values::<S, A>(table, state);
    let mut m = f32::NEG_INFINITY;
    let mut found = false;
    for i in 0..A::ACTION_COUNT {
        if let Some(a) = A::from_index(i) {
            if state.is_admissible(a) {
                m = m.max(q_values[i]);
                found = true;
            }
        }
    }
    if found {
        m
    } else {
        0.0
    }
=======
    get_q_values::<S, A, V>(table, state)
        .iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
}

pub(crate) fn epsilon_greedy_probs<const N: usize>(values: &[f32], epsilon: f32) -> [f32; N] {
    let n = values.len();
    assert!(n <= N);
    let mut probs = [0.0; N];
    if n == 0 {
        return probs;
    }

    let eps = clamp_probability(epsilon);
    let greedy = greedy_index(values);
    let uniform = eps / n as f32;
    for i in 0..n {
        probs[i] = uniform;
    }
    probs[greedy] += 1.0 - eps;
    probs
}

pub(crate) fn softmax_probs<const N: usize>(logits: &[f32]) -> [f32; N] {
    let n = logits.len();
    assert!(n <= N);
    let mut probs = [0.0; N];
    if n == 0 {
        return probs;
    }

    let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut exps = [0.0; N];
    let mut z: f32 = 0.0;
    for i in 0..n {
        exps[i] = (logits[i] - max_logit).exp();
        z += exps[i];
    }

    if z <= 0.0 || !z.is_finite() {
        let val = 1.0 / n as f32;
        for i in 0..n {
            probs[i] = val;
        }
    } else {
        for i in 0..n {
            probs[i] = exps[i] / z;
        }
    }
    probs
}
=======
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
