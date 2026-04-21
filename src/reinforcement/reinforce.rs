use crate::utils::dense_kernel::StaticPackedKeyTable;
use fastrand::Rng;
use std::cell::RefCell;
use std::marker::PhantomData;

use super::*;

pub struct ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    pub(crate) theta: RefCell<StaticPackedKeyTable<S, A::Values, 1024>>,
    pub(crate) learning_rate: f32,
    pub(crate) discount_factor: f32,
    pub(crate) rng: RefCell<Rng>,
    pub(crate) _phantom: PhantomData<A>,
}

impl<S, A> ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
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

        let mut g = 0.0f32;
        let mut theta = self.theta.borrow_mut();

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
        *theta.get(hash_state(&state)).unwrap_or(&A::Values::default())
    }

    pub fn set_exploration_rate(&mut self, _rate: f32) {
        // No-op: REINFORCE uses stochastic policy directly.
    }
}

impl<S, A> Default for ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
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
            state_values.insert(key, weights.as_slice().to_vec());
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

        let mut theta = self.theta.borrow_mut();
        theta.clear();

        for (key, weights_vec) in table.state_values {
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
                marking_mask: 0,
                activities_hash: 0,
            };
            let mut weights = [0.0; 3];
            for (i, &w) in weights_vec.iter().enumerate().take(3) {
                weights[i] = w;
            }
            let _ = theta.insert(hash_state(&state), state, weights);
        }
    }
}

impl<S, A> Agent<S, A> for ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
    fn select_action(&self, state: S) -> A {
        self.select_action(state)
    }

    fn update(&self, state: S, action: A, reward: f32, _next_state: S, _done: bool) {
        self.update_step(state, action, reward);
    }

    fn reset(&self) {}
}

impl<S, A> AgentMeta for ReinforceAgent<S, A>
where
    S: WorkflowState + Copy + Default,
    A: WorkflowAction,
    A::Values: Copy + Default,
{
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
