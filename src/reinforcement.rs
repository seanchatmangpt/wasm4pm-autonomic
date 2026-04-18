//! Ported from knhk/rust/knhk-neural/src/reinforcement.rs
//!
//! Q-Learning, SARSA, Double Q-Learning, Expected SARSA, and REINFORCE
//! for self-optimizing workflows in WASM single-threaded execution.
//!
//! Key WASM changes:
//! - `Arc<RwLock<HashMap>>` replaced with `RefCell<HashMap>`
//! - `Send + Sync` trait bounds removed

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use fastrand::Rng;

pub const DEFAULT_LEARNING_RATE: f32 = 0.1;
pub const DEFAULT_DISCOUNT_FACTOR: f32 = 0.99;
pub const DEFAULT_EXPLORATION_RATE: f32 = 1.0;
pub const DEFAULT_EXPLORATION_DECAY: f32 = 0.995;
pub const REINFORCE_LEARNING_RATE: f32 = 0.01;

/// State for reinforcement learning (must be hashable and cloneable)
pub trait WorkflowState: Clone + Eq + Hash {
    /// State features for function approximation
    fn features(&self) -> Vec<f32>;

    /// Is this a terminal state?
    fn is_terminal(&self) -> bool;
}

/// Action for reinforcement learning
pub trait WorkflowAction: Clone + Eq + Hash {
    /// Total number of possible actions
    const ACTION_COUNT: usize;

    /// Convert to index (0..ACTION_COUNT)
    fn to_index(&self) -> usize;

    /// Convert from index
    fn from_index(idx: usize) -> Option<Self>;
}

#[inline]
fn zeros<A: WorkflowAction>() -> Vec<f32> {
    vec![0.0; A::ACTION_COUNT]
}

#[inline]
fn clamp_probability(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

#[inline]
fn decay_probability(current: f32, decay: f32) -> f32 {
    clamp_probability(current * decay)
}

fn greedy_index(values: &[f32]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn get_q_values<S: WorkflowState, A: WorkflowAction>(
    table: &HashMap<S, Vec<f32>>,
    state: &S,
) -> Vec<f32> {
    table.get(state).cloned().unwrap_or_else(zeros::<A>)
}

fn ensure_state<S: WorkflowState, A: WorkflowAction>(
    table: &mut HashMap<S, Vec<f32>>,
    state: &S,
) {
    table.entry(state.clone()).or_insert_with(zeros::<A>);
}

fn max_q<S: WorkflowState, A: WorkflowAction>(table: &HashMap<S, Vec<f32>>, state: &S) -> f32 {
    get_q_values::<S, A>(table, state)
        .into_iter()
        .fold(f32::NEG_INFINITY, f32::max)
}

fn epsilon_greedy_probs(values: &[f32], epsilon: f32) -> Vec<f32> {
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

fn softmax_probs(logits: &[f32]) -> Vec<f32> {
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

/// Trait for any learning agent
pub trait Agent<S: WorkflowState, A: WorkflowAction> {
    fn select_action(&self, state: &S) -> A;
    fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool);
    fn reset(&self);
}

/// Metadata trait for agent introspection
pub trait AgentMeta {
    fn name(&self) -> &'static str;
    fn exploration_rate(&self) -> f32;
    fn decay_exploration(&mut self);
}

// ---------------------------------------------------------------------------
// Q-Learning
// ---------------------------------------------------------------------------

/// Q-Learning agent: model-free, off-policy
pub struct QLearning<S: WorkflowState, A: WorkflowAction> {
    q_table: RefCell<HashMap<S, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    exploration_rate: f32,
    exploration_decay: f32,
    episodes: RefCell<usize>,
    total_reward: RefCell<f32>,
    rng: RefCell<Rng>,
    _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> QLearning<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(HashMap::new()),
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
            q_table: RefCell::new(HashMap::new()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            episodes: RefCell::new(0),
            total_reward: RefCell::new(0.0),
            rng: RefCell::new(Rng::with_seed(seed)),
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

    #[allow(dead_code)]
    pub fn select_action(&self, state: &S) -> A {
        if self.rng.borrow_mut().f32() < self.exploration_rate {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.best_action(state)
        }
    }

    fn best_action(&self, state: &S) -> A {
        let q_table = self.q_table.borrow();
        let q_values = get_q_values::<S, A>(&q_table, state);
        A::from_index(greedy_index(&q_values)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        let mut q_table = self.q_table.borrow_mut();
        ensure_state::<S, A>(&mut q_table, state);

        let next_val = if done { 0.0 } else { max_q::<S, A>(&q_table, next_state) };

        let action_idx = action.to_index();
        let current_q = q_table[state][action_idx];
        let target = reward + self.discount_factor * next_val;
        q_table.get_mut(state).unwrap()[action_idx] += self.learning_rate * (target - current_q);

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
            .get(state)
            .map(|q_vals| q_vals[action.to_index()])
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

impl<S: WorkflowState, A: WorkflowAction> Default for QLearning<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for QLearning
impl QLearning<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let q_table = self.q_table.borrow();
        let mut state_values = HashMap::new();

        for (state, q_values) in q_table.iter() {
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

        SerializedAgentQTable { agent_type, state_values }
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
            q_table.insert(
                crate::RlState {
                    health_level: h,
                    event_rate_q: e,
                    activity_count_q: a,
                    spc_alert_level: s,
                    drift_status: d,
                    rework_ratio_q: r,
                    circuit_state: c,
                    cycle_phase: p,
                    marking_vec: Vec::new(),
                    recent_activities: Vec::new(),
                },
                q_values,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// SARSA
// ---------------------------------------------------------------------------

/// SARSA agent: model-free, on-policy
///
/// This implementation keeps a pending `(next_state, next_action)` pair captured
/// at action-selection time so that the subsequent update can use the actual
/// on-policy next action.
pub struct SARSAAgent<S: WorkflowState, A: WorkflowAction> {
    q_table: RefCell<HashMap<S, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    exploration_rate: f32,
    exploration_decay: f32,
    pending_next: RefCell<Option<(S, A)>>,
    rng: RefCell<Rng>,
    _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> SARSAAgent<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(HashMap::new()),
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
            q_table: RefCell::new(HashMap::new()),
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
    pub fn epsilon_greedy_action(&self, state: &S, epsilon: f32) -> A {
        let eps = clamp_probability(epsilon);
        if self.rng.borrow_mut().f32() < eps {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.greedy_action(state)
        }
    }

    fn greedy_action(&self, state: &S) -> A {
        let q_table = self.q_table.borrow();
        let q_vals = get_q_values::<S, A>(&q_table, state);
        A::from_index(greedy_index(&q_vals)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update_with_next_action(
        &self,
        state: &S,
        action: &A,
        reward: f32,
        next_state: &S,
        next_action: &A,
        done: bool,
    ) {
        let mut q_table = self.q_table.borrow_mut();
        ensure_state::<S, A>(&mut q_table, state);

        let next_q = if done {
            0.0
        } else {
            q_table
                .get(next_state)
                .map(|q_vals| q_vals[next_action.to_index()])
                .unwrap_or(0.0)
        };

        let action_idx = action.to_index();
        let current_q = q_table[state][action_idx];
        let target = reward + self.discount_factor * next_q;
        q_table.get_mut(state).unwrap()[action_idx] += self.learning_rate * (target - current_q);
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
        let mut state_values = HashMap::new();

        for (state, q_values) in q_table.iter() {
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

        SerializedAgentQTable { agent_type, state_values }
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
            q_table.insert(
                crate::RlState {
                    health_level: h,
                    event_rate_q: e,
                    activity_count_q: a,
                    spc_alert_level: s,
                    drift_status: d,
                    rework_ratio_q: r,
                    circuit_state: c,
                    cycle_phase: p,
                    marking_vec: Vec::new(),
                    recent_activities: Vec::new(),
                },
                q_values,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Double Q-Learning
// ---------------------------------------------------------------------------

pub struct DoubleQLearning<S: WorkflowState, A: WorkflowAction> {
    q_a: RefCell<HashMap<S, Vec<f32>>>,
    q_b: RefCell<HashMap<S, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    exploration_rate: f32,
    exploration_decay: f32,
    rng: RefCell<Rng>,
    _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> DoubleQLearning<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_a: RefCell::new(HashMap::new()),
            q_b: RefCell::new(HashMap::new()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            q_a: RefCell::new(HashMap::new()),
            q_b: RefCell::new(HashMap::new()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            rng: RefCell::new(Rng::with_seed(seed)),
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

    #[allow(dead_code)]
    pub fn select_action(&self, state: &S) -> A {
        if self.rng.borrow_mut().f32() < self.exploration_rate {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.greedy_action(state)
        }
    }

    fn greedy_action(&self, state: &S) -> A {
        let qa = self.q_a.borrow();
        let qb = self.q_b.borrow();

        let va = get_q_values::<S, A>(&qa, state);
        let vb = get_q_values::<S, A>(&qb, state);

        let mut merged = vec![0.0; A::ACTION_COUNT];
        for i in 0..A::ACTION_COUNT {
            merged[i] = va[i] + vb[i];
        }

        A::from_index(greedy_index(&merged)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        let mut qa = self.q_a.borrow_mut();
        let mut qb = self.q_b.borrow_mut();

        ensure_state::<S, A>(&mut qa, state);
        ensure_state::<S, A>(&mut qb, state);

        let action_idx = action.to_index();

        if self.rng.borrow_mut().bool() {
            let next_vals = get_q_values::<S, A>(&qa, next_state);
            let best_next_idx = greedy_index(&next_vals);
            let next_q = if done {
                0.0
            } else {
                qb.get(next_state)
                    .map(|vals| vals[best_next_idx])
                    .unwrap_or(0.0)
            };

            let current = qa[state][action_idx];
            let target = reward + self.discount_factor * next_q;
            qa.get_mut(state).unwrap()[action_idx] += self.learning_rate * (target - current);
        } else {
            let next_vals = get_q_values::<S, A>(&qb, next_state);
            let best_next_idx = greedy_index(&next_vals);
            let next_q = if done {
                0.0
            } else {
                qa.get(next_state)
                    .map(|vals| vals[best_next_idx])
                    .unwrap_or(0.0)
            };

            let current = qb[state][action_idx];
            let target = reward + self.discount_factor * next_q;
            qb.get_mut(state).unwrap()[action_idx] += self.learning_rate * (target - current);
        }
    }

    #[allow(dead_code)]
    pub fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = clamp_probability(rate);
    }

    #[allow(dead_code)]
    pub fn get_exploration_rate(&self) -> f32 {
        self.exploration_rate
    }
}

impl<S: WorkflowState, A: WorkflowAction> Default for DoubleQLearning<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for DoubleQLearning
impl DoubleQLearning<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let qa = self.q_a.borrow();
        let mut state_values = HashMap::new();

        for (state, q_values) in qa.iter() {
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

        SerializedAgentQTable { agent_type, state_values }
    }

    #[allow(dead_code)]
    pub fn restore_from_serialized(
        &self,
        table: crate::rl_state_serialization::SerializedAgentQTable,
    ) {
        use crate::rl_state_serialization::decode_rl_state_key;

        let mut qa = self.q_a.borrow_mut();
        let mut qb = self.q_b.borrow_mut();
        qa.clear();
        qb.clear();

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
                marking_vec: Vec::new(),
                recent_activities: Vec::new(),
            };
            qa.insert(state.clone(), q_values.clone());
            qb.insert(state, q_values);
        }
    }
}

// ---------------------------------------------------------------------------
// Expected SARSA
// ---------------------------------------------------------------------------

pub struct ExpectedSARSAAgent<S: WorkflowState, A: WorkflowAction> {
    q_table: RefCell<HashMap<S, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    exploration_rate: f32,
    exploration_decay: f32,
    rng: RefCell<Rng>,
    _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> ExpectedSARSAAgent<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            q_table: RefCell::new(HashMap::new()),
            learning_rate: DEFAULT_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            q_table: RefCell::new(HashMap::new()),
            learning_rate: lr,
            discount_factor: df,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            exploration_decay: DEFAULT_EXPLORATION_DECAY,
            rng: RefCell::new(Rng::with_seed(seed)),
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

    #[allow(dead_code)]
    pub fn select_action(&self, state: &S) -> A {
        if self.rng.borrow_mut().f32() < self.exploration_rate {
            let idx = self.rng.borrow_mut().usize(..A::ACTION_COUNT);
            A::from_index(idx).unwrap()
        } else {
            self.greedy_action(state)
        }
    }

    fn greedy_action(&self, state: &S) -> A {
        let q_table = self.q_table.borrow();
        let q_vals = get_q_values::<S, A>(&q_table, state);
        A::from_index(greedy_index(&q_vals)).unwrap()
    }

    #[allow(dead_code)]
    pub fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        let expected_next = if done {
            0.0
        } else {
            let q_table = self.q_table.borrow();
            let q_vals = get_q_values::<S, A>(&q_table, next_state);
            drop(q_table);

            let probs = epsilon_greedy_probs(&q_vals, self.exploration_rate);
            q_vals
                .iter()
                .zip(probs.iter())
                .map(|(q, p)| q * p)
                .sum::<f32>()
        };

        let mut q_table = self.q_table.borrow_mut();
        ensure_state::<S, A>(&mut q_table, state);

        let action_idx = action.to_index();
        let current_q = q_table[state][action_idx];
        let target = reward + self.discount_factor * expected_next;
        q_table.get_mut(state).unwrap()[action_idx] += self.learning_rate * (target - current_q);
    }

    #[allow(dead_code)]
    pub fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }

    pub fn set_exploration_rate(&mut self, rate: f32) {
        self.exploration_rate = clamp_probability(rate);
    }

    #[allow(dead_code)]
    pub fn get_exploration_rate(&self) -> f32 {
        self.exploration_rate
    }
}

impl<S: WorkflowState, A: WorkflowAction> Default for ExpectedSARSAAgent<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for ExpectedSARSAAgent
impl ExpectedSARSAAgent<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let q_table = self.q_table.borrow();
        let mut state_values = HashMap::new();

        for (state, q_values) in q_table.iter() {
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

        SerializedAgentQTable { agent_type, state_values }
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
            q_table.insert(
                crate::RlState {
                    health_level: h,
                    event_rate_q: e,
                    activity_count_q: a,
                    spc_alert_level: s,
                    drift_status: d,
                    rework_ratio_q: r,
                    circuit_state: c,
                    cycle_phase: p,
                    marking_vec: Vec::new(),
                    recent_activities: Vec::new(),
                },
                q_values,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// REINFORCE
// ---------------------------------------------------------------------------

pub struct ReinforceAgent<S: WorkflowState, A: WorkflowAction> {
    theta: RefCell<HashMap<S, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    rng: RefCell<Rng>,
    _phantom: PhantomData<A>,
}

impl<S: WorkflowState, A: WorkflowAction> ReinforceAgent<S, A> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            theta: RefCell::new(HashMap::new()),
            learning_rate: REINFORCE_LEARNING_RATE,
            discount_factor: DEFAULT_DISCOUNT_FACTOR,
            rng: RefCell::new(Rng::new()),
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_seed(lr: f32, df: f32, seed: u64) -> Self {
        Self {
            theta: RefCell::new(HashMap::new()),
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
    pub fn select_action(&self, state: &S) -> A {
        let theta = self.theta.borrow();
        let weights = theta.get(state).cloned().unwrap_or_else(zeros::<A>);
        drop(theta);

        let probs = softmax_probs(&weights);
        let u = self.rng.borrow_mut().f32();
        let mut acc = 0.0;

        for (idx, p) in probs.iter().enumerate() {
            acc += *p;
            if u <= acc {
                return A::from_index(idx).unwrap();
            }
        }

        A::from_index(A::ACTION_COUNT - 1).unwrap()
    }

    #[allow(dead_code)]
    pub fn update_from_trajectory(&self, trajectory: &[(S, A, f32)]) {
        let n = trajectory.len();
        if n == 0 {
            return;
        }

        let mut returns = vec![0.0f32; n];
        let mut g = 0.0f32;
        for i in (0..n).rev() {
            g = trajectory[i].2 + self.discount_factor * g;
            returns[i] = g;
        }

        let mut theta = self.theta.borrow_mut();

        for (t, (state, action, _)) in trajectory.iter().enumerate() {
            ensure_state::<S, A>(&mut theta, state);
            let logits = theta.get(state).cloned().unwrap_or_else(zeros::<A>);
            let probs = softmax_probs(&logits);
            let a_idx = action.to_index();
            let g_t = returns[t];

            let weights = theta.get_mut(state).unwrap();
            for j in 0..A::ACTION_COUNT {
                let grad = if j == a_idx { 1.0 - probs[j] } else { -probs[j] };
                weights[j] += self.learning_rate * g_t * grad;
            }
        }
    }

    #[allow(dead_code)]
    pub fn update_step(&self, state: &S, action: &A, reward: f32) {
        self.update_from_trajectory(&[(state.clone(), action.clone(), reward)]);
    }

    #[allow(dead_code)]
    pub fn get_policy_weights(&self, state: &S) -> Vec<f32> {
        let theta = self.theta.borrow();
        theta.get(state).cloned().unwrap_or_else(zeros::<A>)
    }

    pub fn set_exploration_rate(&mut self, _rate: f32) {
        // No-op: REINFORCE uses stochastic policy directly.
    }
}

impl<S: WorkflowState, A: WorkflowAction> Default for ReinforceAgent<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

// Serialization support for ReinforceAgent
impl ReinforceAgent<crate::RlState, crate::RlAction> {
    #[allow(dead_code)]
    pub fn export_as_serialized(
        &self,
        agent_type: u8,
    ) -> crate::rl_state_serialization::SerializedAgentQTable {
        use crate::rl_state_serialization::{encode_rl_state_key, SerializedAgentQTable};

        let theta = self.theta.borrow();
        let mut state_values = HashMap::new();

        for (state, weights) in theta.iter() {
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
            state_values.insert(key, weights.clone());
        }

        SerializedAgentQTable { agent_type, state_values }
    }

    #[allow(dead_code)]
    pub fn restore_from_serialized(
        &self,
        table: crate::rl_state_serialization::SerializedAgentQTable,
    ) {
        use crate::rl_state_serialization::decode_rl_state_key;

        let mut theta = self.theta.borrow_mut();
        theta.clear();

        for (key, weights) in table.state_values {
            let (h, e, a, s, d, r, c, p) = decode_rl_state_key(key);
            theta.insert(
                crate::RlState {
                    health_level: h,
                    event_rate_q: e,
                    activity_count_q: a,
                    spc_alert_level: s,
                    drift_status: d,
                    rework_ratio_q: r,
                    circuit_state: c,
                    cycle_phase: p,
                    marking_vec: Vec::new(),
                    recent_activities: Vec::new(),
                },
                weights,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Agent trait implementations
// ---------------------------------------------------------------------------

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for QLearning<S, A> {
    fn select_action(&self, state: &S) -> A {
        QLearning::select_action(self, state)
    }

    fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        QLearning::update(self, state, action, reward, next_state, done)
    }

    fn reset(&self) {}
}

impl<S: WorkflowState, A: WorkflowAction> AgentMeta for QLearning<S, A> {
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

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for SARSAAgent<S, A> {
    fn select_action(&self, state: &S) -> A {
        let mut pending = self.pending_next.borrow_mut();
        if let Some((ref s, ref a)) = *pending {
            if s == state {
                return a.clone();
            }
        }
        let action = self.epsilon_greedy_action(state, self.exploration_rate);
        *pending = Some((state.clone(), action.clone()));
        action
    }

    fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        if done {
            self.update_with_next_action(state, action, reward, next_state, action, true);
            return;
        }

        let mut pending = self.pending_next.borrow_mut();
        let next_action = match pending.take() {
            Some((pending_state, pending_action)) if pending_state == *next_state => pending_action,
            _ => self.epsilon_greedy_action(next_state, self.exploration_rate),
        };
        // Re-store the next_action so the subsequent select_action uses it
        *pending = Some((next_state.clone(), next_action.clone()));
        drop(pending);

        self.update_with_next_action(state, action, reward, next_state, &next_action, false);
    }

    fn reset(&self) {
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

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for DoubleQLearning<S, A> {
    fn select_action(&self, state: &S) -> A {
        DoubleQLearning::select_action(self, state)
    }

    fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        DoubleQLearning::update(self, state, action, reward, next_state, done)
    }

    fn reset(&self) {}
}

impl<S: WorkflowState, A: WorkflowAction> AgentMeta for DoubleQLearning<S, A> {
    fn name(&self) -> &'static str {
        "DoubleQLearning"
    }

    fn exploration_rate(&self) -> f32 {
        self.exploration_rate
    }

    fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }
}

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for ExpectedSARSAAgent<S, A> {
    fn select_action(&self, state: &S) -> A {
        ExpectedSARSAAgent::select_action(self, state)
    }

    fn update(&self, state: &S, action: &A, reward: f32, next_state: &S, done: bool) {
        ExpectedSARSAAgent::update(self, state, action, reward, next_state, done)
    }

    fn reset(&self) {}
}

impl<S: WorkflowState, A: WorkflowAction> AgentMeta for ExpectedSARSAAgent<S, A> {
    fn name(&self) -> &'static str {
        "ExpectedSARSA"
    }

    fn exploration_rate(&self) -> f32 {
        self.exploration_rate
    }

    fn decay_exploration(&mut self) {
        self.exploration_rate = decay_probability(self.exploration_rate, self.exploration_decay);
    }
}

impl<S: WorkflowState, A: WorkflowAction> Agent<S, A> for ReinforceAgent<S, A> {
    fn select_action(&self, state: &S) -> A {
        ReinforceAgent::select_action(self, state)
    }

    fn update(&self, state: &S, action: &A, reward: f32, _next_state: &S, _done: bool) {
        self.update_step(state, action, reward);
    }

    fn reset(&self) {}
}

impl<S: WorkflowState, A: WorkflowAction> AgentMeta for ReinforceAgent<S, A> {
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

// Tests consolidated in tests/reinforcement_tests.rs