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

/// State for reinforcement learning (must be hashable and copyable)
pub trait WorkflowState: Clone + Copy + Eq + Hash {
    /// State features for function approximation (zero-heap)
    fn features(&self) -> [f32; 16];

    /// Is this a terminal state?
    fn is_terminal(&self) -> bool;
}

/// Action for reinforcement learning (must be copyable)
pub trait WorkflowAction: Clone + Copy + Eq + Hash {
    /// Total number of possible actions
    const ACTION_COUNT: usize;

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

/*
#[inline]
pub(crate) fn zeros<A: WorkflowAction>() -> Vec<f32> {
    vec![0.0; A::ACTION_COUNT]
}
*/

#[inline]
pub(crate) fn clamp_probability(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

#[inline]
pub(crate) fn decay_probability(current: f32, decay: f32) -> f32 {
    clamp_probability(current * decay)
}

pub(crate) fn greedy_index(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

#[inline]
pub(crate) fn hash_state<S: Hash>(state: &S) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    state.hash(&mut hasher);
    hasher.finish()
}

pub const ACTION_MAX_LIMIT: usize = 8;
pub type QArray = [f32; ACTION_MAX_LIMIT];

pub(crate) fn get_q_values<'a, S, A>(table: &'a PackedKeyTable<S, QArray>, state: &S) -> &'a [f32]
where
    S: WorkflowState,
    A: WorkflowAction,
{
    static ZEROS: QArray = [0.0; ACTION_MAX_LIMIT];
    table
        .get(hash_state(state))
        .map(|v| &v[..A::ACTION_COUNT])
        .unwrap_or(&ZEROS[..A::ACTION_COUNT])
}

pub(crate) fn ensure_state<S, A>(table: &mut PackedKeyTable<S, QArray>, state: S)
where
    S: WorkflowState,
    A: WorkflowAction,
{
    let h = hash_state(&state);
    if table.get(h).is_none() {
        table.insert(h, state, [0.0; ACTION_MAX_LIMIT]);
    }
}

pub(crate) fn max_q<S, A>(table: &PackedKeyTable<S, QArray>, state: &S) -> f32
where
    S: WorkflowState,
    A: WorkflowAction,
{
    get_q_values::<S, A>(table, state)
        .iter()
        .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
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
