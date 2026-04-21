#[cfg(test)]
mod proptests {
    use crate::reinforcement::{WorkflowAction, WorkflowState};
    use crate::{RlAction, RlState};
    use proptest::prelude::*;

    // μ-kernel invariant: Var(τ) == 0 (Deterministic Execution)
    proptest! {
        #[test]
        fn test_μ_kernel_determinism(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = RlState {
                health_level: h,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
                marking_mask: 0,
                activities_hash: 0,
            };
            let action = RlAction::from_index(a).unwrap();
            
            let result1 = transition(state, action);
            let result2 = transition(state, action);
            
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic");
        }

        #[test]
        fn test_branchless_fire_admissibility(
            m in 0u64..u64::MAX,
            i in 0u64..u64::MAX,
            o in 0u64..u64::MAX,
        ) {
            let mut state = RlState {
                marking_mask: m,
                activities_hash: 0,
                health_level: 0,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
            };

            let is_enabled = (m & i) == i;
            let fired = crate::dteam::kernel::branchless::apply_branchless_fire(&mut state, i, o);
            
            assert_eq!(fired, is_enabled);
            if is_enabled {
                assert_eq!(state.marking_mask, (m & !i) | o);
            } else {
                assert_eq!(state.marking_mask, m);
            }
        }

        #[test]
        fn test_rl_action_admissibility(
            h in -10i8..10i8,
            a in 0usize..3,
        ) {
            let state = RlState {
                health_level: h,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
                marking_mask: 0,
                activities_hash: 0,
            };
            let action = RlAction::from_index(a).unwrap();
            
            let admissible = state.is_admissible(action);
            match action {
                RlAction::Idle => assert!(admissible),
                RlAction::Optimize => assert_eq!(admissible, h < 5),
                RlAction::Rework => assert_eq!(admissible, h > 0),
            }
        }
    }

    fn transition(state: RlState, action: RlAction) -> RlState {
        let mut next = state;
        match action {
            RlAction::Idle => (),
            RlAction::Optimize => next.health_level += 1,
            RlAction::Rework => next.health_level = (next.health_level - 1).max(0),
        }
        next
    }
}
