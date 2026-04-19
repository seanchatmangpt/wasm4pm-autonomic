//! Core traits and utilities for reinforcement learning
//! in single-threaded WASM environments.

use std::hash::Hash;
use std::collections::HashMap;
use std::hash::BuildHasher;

pub const DEFAULT_LEARNING_RATE: f32 = 0.1;
pub const DEFAULT_DISCOUNT_FACTOR: f32 = 0.99;
pub const DEFAULT_EXPLORATION_RATE: f32 = 1.0;
pub const DEFAULT_EXPLORATION_DECAY: f32 = 0.995;
pub const REINFORCE_LEARNING_RATE: f32 = 0.01;

pub mod q_learning;
pub mod sarsa;
pub mod double_q;
pub mod expected_sarsa;
pub mod reinforce;

pub use q_learning::QLearning;
pub use sarsa::SARSAAgent;
pub use double_q::DoubleQLearning;
pub use expected_sarsa::ExpectedSARSAAgent;
pub use reinforce::ReinforceAgent;

/// State for reinforcement learning (must be hashable and copyable)
pub trait WorkflowState: Clone + Copy + Eq + Hash {
    /// State features for function approximation
    fn features(&self) -> Vec<f32>;

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

#[inline]
pub(crate) fn zeros<A: WorkflowAction>() -> Vec<f32> {
    vec![0.0; A::ACTION_COUNT]
}

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

pub(crate) fn get_q_values<S, A, H>(
    table: &HashMap<S, Vec<f32>, H>,
    state: &S,
) -> Vec<f32> 
where 
    S: WorkflowState, 
    A: WorkflowAction,
    H: BuildHasher
{
    table.get(state).cloned().unwrap_or_else(zeros::<A>)
}

pub(crate) fn ensure_state<S, A, H>(
    table: &mut HashMap<S, Vec<f32>, H>,
    state: S,
) 
where 
    S: WorkflowState, 
    A: WorkflowAction,
    H: BuildHasher + Default
{
    table.entry(state).or_insert_with(zeros::<A>);
}

pub(crate) fn max_q<S, A, H>(table: &HashMap<S, Vec<f32>, H>, state: &S) -> f32 
where 
    S: WorkflowState, 
    A: WorkflowAction,
    H: BuildHasher
{
    get_q_values::<S, A, H>(table, state)
        .into_iter()
        .fold(f32::NEG_INFINITY, f32::max)
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
