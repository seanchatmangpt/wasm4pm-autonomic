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
    /// State features for function approximation
    fn features(&self) -> Vec<f32>;

    /// Is this a terminal state?
    fn is_terminal(&self) -> bool;

    /// Is this action admissible in the current state?
    fn is_admissible<A: WorkflowAction>(&self, _action: A) -> bool {
        true
    }
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
    fn update(&self, state: S, action: A, reward: f32, next_state: S, done: bool);
    fn reset(&self);
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

pub(crate) fn get_q_values<'a, S, A>(table: &'a PackedKeyTable<S, Vec<f32>>, state: &S) -> &'a [f32]
where
    S: WorkflowState,
    A: WorkflowAction,
{
    static ZEROS: [f32; 256] = [0.0; 256];
    table
        .get(hash_state(state))
        .map(|v| v.as_slice())
        .unwrap_or(&ZEROS[..A::ACTION_COUNT])
}

pub(crate) fn ensure_state<S, A>(table: &mut PackedKeyTable<S, Vec<f32>>, state: S)
where
    S: WorkflowState,
    A: WorkflowAction,
{
    let h = hash_state(&state);
    if table.get(h).is_none() {
        table.insert(h, state, vec![0.0; A::ACTION_COUNT]);
    }
}

pub(crate) fn max_q<S, A>(table: &PackedKeyTable<S, Vec<f32>>, state: &S) -> f32
where
    S: WorkflowState,
    A: WorkflowAction,
{
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
}

pub(crate) fn epsilon_greedy_probs(values: &[f32], epsilon: f32) -> Vec<f32> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }

    let eps = clamp_probability(epsilon);
    let greedy = greedy_index(values);
    let uniform = eps / n as f32;
    let mut probs = vec![uniform; n];
    probs[greedy] += 1.0 - eps;
    probs
}

pub(crate) fn softmax_probs(logits: &[f32]) -> Vec<f32> {
    if logits.is_empty() {
        return Vec::new();
    }

    let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&x| (x - max_logit).exp()).collect();
    let z: f32 = exps.iter().sum();

    if z <= 0.0 || !z.is_finite() {
        vec![1.0 / logits.len() as f32; logits.len()]
    } else {
        exps.into_iter().map(|e| e / z).collect()
    }
}
