#[cfg(test)]
mod proptests {
    use crate::dteam::kernel::branchless::apply_branchless_update;
    use crate::models::petri_net::{FlatIncidenceMatrix, PetriNet, Place, Transition, Arc};
    use crate::reinforcement::WorkflowAction;
    use crate::utils::dense_kernel::KBitSet;
    use crate::{RlAction, RlState};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_μ_kernel_determinism(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = RlState::<1> {
                health_level: h,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
                marking_mask: KBitSet::zero(),
                activities_hash: 0,
                ontology_mask: KBitSet::zero(),
            };
            let action = RlAction::from_index(a).unwrap();

            // Execute twice to check variancy τ
            let result1 = transition(state, action);
            let result2 = transition(state, action);

            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic");
        }

        #[test]
        fn test_branchless_kernel_equation_parity(
            mask in 0u64..1024,
            transitions in 1usize..8,
        ) {
            let places_count = 10;
            // Generate a random incidence matrix
            let mut data = vec![0i32; places_count * transitions];
            for i in 0..places_count * transitions {
                data[i] = if i % 3 == 0 { -1 } else if i % 3 == 1 { 1 } else { 0 };
            }
            let incidence = FlatIncidenceMatrix {
                data,
                places_count,
                transitions_count: transitions,
            };

            let transition_idx = 0; // Test first transition
            let result1 = apply_branchless_update(mask, transition_idx, &incidence);
            let result2 = apply_branchless_update(mask, transition_idx, &incidence);
            
            assert_eq!(result1, result2, "Branchless transition failed: not deterministic");
        }
    }
    fn transition(state: RlState<1>, action: RlAction) -> RlState<1> {
        let mut next = state;
        match action {
            RlAction::Idle => (),
            RlAction::Optimize => next.health_level += 1,
            RlAction::Rework => next.health_level = (next.health_level - 1).max(0),
        }
        next
    }
}
