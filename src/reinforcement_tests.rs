#[cfg(test)]
mod tests {
    use crate::reinforcement::{
        Agent, DoubleQLearning, ExpectedSARSAAgent, QLearning, ReinforceAgent, SARSAAgent,
    };
    use crate::utils::dense_kernel::KBitSet;
    use crate::{RlAction, RlState};
    use crate::utils::perturbation::Perturbator;
    use proptest::prelude::*;

    const MAX_STEPS: usize = 100;
    const GOAL_STATE_DEFAULT: i32 = 5;
    const GOAL_STATE_REINFORCE: i32 = 3;
    const EPISODES_STANDARD: usize = 1000;
    const EPISODES_EXTENDED: usize = 2000;
    const EVAL_EPISODES: usize = 100;
    const DECAY_INTERVAL: usize = 200;
    const AVG_REWARD_THRESHOLD: f32 = 0.5;

    proptest! {
        #[test]
        fn test_perturbator_determinism(seed: u64, mask: u64, intensity: u64) {
            let mut p1 = Perturbator::new(seed);
            let mut p2 = Perturbator::new(seed);
            
            let m1 = p1.perturb_mask(mask, intensity);
            let m2 = p2.perturb_mask(mask, intensity);
            
            assert_eq!(m1, m2, "Perturbator must be deterministic for a given seed");
        }
    }

    fn create_state(h: i32) -> RlState<1> {
        RlState::<1> {
            health_level: h as i8,
            event_rate_q: 0,
            activity_count_q: 0,
            spc_alert_level: 0,
            drift_status: 0,
            rework_ratio_q: 0,
            circuit_state: 0,
            cycle_phase: 0,
<<<<<<< HEAD
            marking_mask: KBitSet::zero(),
=======
            marking_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
            activities_hash: 0,
            ontology_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
<<<<<<< HEAD
                universe: None,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
        }
    }

    /// A simple corridor environment where the agent starts at 0 and must reach GOAL_STATE.
    /// State is represented by health_level.
    /// Actions: Idle (Stay), Optimize (Right), Rework (Left).
    fn run_corridor<T: Agent<RlState<1>, RlAction>>(
        agent: &mut T,
        episodes: usize,
        goal_state: i32,
    ) -> f32 {
        let mut total_reward = 0.0;
        for _ in 0..episodes {
            agent.reset();
            let mut state = create_state(0);
            let mut steps = 0;
            while (state.health_level as i32) < goal_state && steps < MAX_STEPS {
                let action = agent.select_action(state);
                let next_h = match action {
                    RlAction::Idle => state.health_level,
                    RlAction::Optimize => state.health_level + 1,
                    RlAction::Rework => (state.health_level - 1).max(0),
                };
                let next_state = create_state(next_h as i32);
                let done = (next_h as i32) >= goal_state;
                let reward = if done { 1.0 } else { 0.0 };

                agent.update(state, action, reward, next_state, done);

                state = next_state;
                total_reward += reward;
                steps += 1;
            }
        }
        total_reward / episodes as f32
    }

    #[test]
    fn test_q_learning_convergence() {
        let mut agent = QLearning::with_hyperparams(0.1, 0.9, 0.5);
        let avg_reward = run_corridor(&mut agent, EPISODES_STANDARD, GOAL_STATE_DEFAULT);
        assert!(
            avg_reward > AVG_REWARD_THRESHOLD,
            "Q-Learning should learn to reach the goal (avg_reward: {})",
            avg_reward
        );
    }

    #[test]
    fn test_sarsa_convergence() {
        let mut agent = SARSAAgent::new();

        // Training in deterministic SARSA (increased episodes to ensure convergence)
        for _ in 0..(EPISODES_EXTENDED * 5) {
            run_corridor(&mut agent, 1, GOAL_STATE_DEFAULT);
        }

        agent.set_exploration_rate(0.0);
        let avg_reward = run_corridor(&mut agent, EVAL_EPISODES, GOAL_STATE_DEFAULT);
        assert!(
            avg_reward > AVG_REWARD_THRESHOLD,
            "SARSA should learn to reach the goal (avg_reward: {})",
            avg_reward
        );
    }


    #[test]
    fn test_double_q_learning_convergence() {
        let mut agent = DoubleQLearning::with_hyperparams(0.1, 0.9, 0.5);

        // Manual decay during training
        for ep in 0..EPISODES_EXTENDED {
            run_corridor(&mut agent, 1, GOAL_STATE_DEFAULT);
            if ep % DECAY_INTERVAL == 0 {
                agent.decay_exploration();
            }
        }

        agent.set_exploration_rate(0.0);
        let avg_reward = run_corridor(&mut agent, EVAL_EPISODES, GOAL_STATE_DEFAULT);
        assert!(
            avg_reward > AVG_REWARD_THRESHOLD,
            "Double Q-Learning should learn to reach the goal (avg_reward: {})",
            avg_reward
        );
    }

    #[test]
    fn test_expected_sarsa_convergence() {
        let mut agent = ExpectedSARSAAgent::with_hyperparams(0.1, 0.9, 0.5);
        let avg_reward = run_corridor(&mut agent, EPISODES_STANDARD, GOAL_STATE_DEFAULT);
        assert!(
            avg_reward > AVG_REWARD_THRESHOLD,
            "Expected SARSA should learn to reach the goal (avg_reward: {})",
            avg_reward
        );
    }

    #[test]
    fn test_reinforce_convergence() {
        let mut agent = ReinforceAgent::with_hyperparams(0.1, 0.9);
        let avg_reward = run_corridor(&mut agent, EPISODES_STANDARD, GOAL_STATE_REINFORCE);
        assert!(
            avg_reward > AVG_REWARD_THRESHOLD,
            "REINFORCE should learn to reach the goal (avg_reward: {})",
            avg_reward
        );
    }

    #[test]
    fn test_negative_reward_avoidance() {
        let mut agent = QLearning::with_hyperparams(0.1, 0.9, 0.1);
        for _ in 0..200 {
            let state = create_state(0);
            agent.update(state, RlAction::Optimize, -10.0, create_state(1), true);
            agent.update(state, RlAction::Idle, -1.0, create_state(2), true);
        }
        let q_bad = agent.get_q_value(&create_state(0), &RlAction::Optimize);
        let q_good = agent.get_q_value(&create_state(0), &RlAction::Idle);
        assert!(
            q_good > q_bad,
            "Agent should prefer -1 reward over -10 reward (good: {}, bad: {})",
            q_good,
            q_bad
        );
    }

    #[test]
    fn test_double_q_serialization_roundtrip() {
        let mut agent = DoubleQLearning::<RlState<1>, RlAction>::new();
        let state = create_state(42);

        for _ in 0..100 {
            agent.update(state, RlAction::Optimize, 100.0, create_state(43), true);
        }

        let serialized = agent.export_as_serialized(3);
        let mut new_agent = DoubleQLearning::<RlState<1>, RlAction>::new();
        new_agent.restore_from_serialized(serialized);

        new_agent.set_exploration_rate(0.0);
        let selected = new_agent.select_action(state);
        assert_eq!(selected, RlAction::Optimize);
    }

<<<<<<< HEAD
    // --- SARSA Rigor Tests ---

    #[test]
    fn test_sarsa_zero_variancy() {
        let agent = SARSAAgent::<RlState, RlAction>::new();
        let state = create_state(0);
        
        *agent.episode_count.borrow_mut() = 123;
        let a1 = agent.select_action(state);
        let a2 = agent.select_action(state);
        assert_eq!(a1, a2, "SARSA agent selection is non-deterministic!");
        
        let a3 = agent.select_action(state);
        assert_eq!(a1, a3, "SARSA agent selection is unstable!");
    }

    #[test]
    fn test_sarsa_exploration_coverage() {
        let agent = SARSAAgent::<RlState, RlAction>::new();
        let state = create_state(0);
        let mut picked = std::collections::HashSet::new();
        
        // Ensure that through rotation, we see all actions
        for ep in 0..20 {
            *agent.episode_count.borrow_mut() = ep;
            picked.insert(agent.select_action(state));
        }
        
        assert_eq!(picked.len(), 3, "SARSA rotation failed to cover action space");
=======
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_ktier_marking_admissibility(
            idx in 0usize..1024,
        ) {
            let mut mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
            let _ = mask.set(idx);
            assert!(mask.contains(idx));
            assert_eq!(mask.pop_count(), 1);
        }
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    }
}
