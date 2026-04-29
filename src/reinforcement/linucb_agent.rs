
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Eq, PartialEq, Hash)]
    struct SimpleState(u32);

    impl WorkflowState for SimpleState {
        fn features(&self) -> [f32; 16] {
            let mut f = [0.0; 16];
            f[0] = self.0 as f32;
            f
        }

        fn is_terminal(&self) -> bool {
            self.0 > 100
        }
    }

    #[derive(Clone, Copy, Eq, PartialEq, Hash)]
    struct SimpleAction(usize);

    impl WorkflowAction for SimpleAction {
        const ACTION_COUNT: usize = 2;

        fn to_index(&self) -> usize {
            self.0
        }

        fn from_index(idx: usize) -> Option<Self> {
            if idx < Self::ACTION_COUNT {
                Some(SimpleAction(idx))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_linucb_agent_basic_operations() {
        // D=4, D2=16 (D*D), so this agent uses a 4-dimensional context
        let mut agent: LinUcbAgent<4, 16, SimpleState, SimpleAction> = LinUcbAgent::new(1.0);

        let state = SimpleState(5);
        let action = agent.select_action(state);
        assert!(action.to_index() < 2);

        agent.update(state, action, 1.0, SimpleState(6), false);
        agent.reset();

        let action2 = agent.select_action(state);
        assert!(action2.to_index() < 2);
    }
}
