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

/// Fixed-size array of action values to eliminate heap allocations.
pub trait ActionArray: Default + Clone + Copy + Send + Sync {
    fn get(&self, index: usize) -> f32;
    fn set(&mut self, index: usize, value: f32);
    fn len(&self) -> usize;
    fn as_slice(&self) -> &[f32];
    fn as_mut_slice(&mut self) -> &mut [f32];
}

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
