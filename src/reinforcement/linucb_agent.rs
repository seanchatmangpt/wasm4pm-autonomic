
//! Agent integration for reinforcement learning agents.

use crate::ml::linucb::LinUcb;
use crate::reinforcement::{Agent, WorkflowAction, WorkflowState};

pub struct LinUcbAgent<const D: usize, const D2: usize, S, A> {
    pub model: LinUcb<D, D2>,
    _phantom_s: std::marker::PhantomData<S>,
    _phantom_a: std::marker::PhantomData<A>,
}

impl<const D: usize, const D2: usize, S, A> LinUcbAgent<D, D2, S, A> {
    pub fn new(alpha: f32) -> Self {
        Self {
            model: LinUcb::new(alpha),
            _phantom_s: std::marker::PhantomData,
            _phantom_a: std::marker::PhantomData,
        }
    }
}

impl<const D: usize, const D2: usize, S, A> Agent<S, A> for LinUcbAgent<D, D2, S, A>
where
    S: WorkflowState,
    A: WorkflowAction,
{
    fn select_action(&self, state: S) -> A {
        let features = state.features();
        let mut context = [0.0; D];
        for i in 0..D.min(features.len()) {
            context[i] = features[i];
        }
        
        let idx = self.model.select_action(&context, A::ACTION_COUNT);
        A::from_index(idx).unwrap_or_else(|| A::from_index(0).unwrap())
    }

    fn update(&mut self, state: S, _action: A, reward: f32, _next_state: S, _done: bool) {
        let features = state.features();
        let mut context = [0.0; D];
        for i in 0..D.min(features.len()) {
            context[i] = features[i];
        }
        self.model.update(&context, reward);
    }

    fn reset(&mut self) {
        self.model = LinUcb::new(self.model.alpha);
    }
}
