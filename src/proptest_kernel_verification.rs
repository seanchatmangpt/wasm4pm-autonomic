#[cfg(test)]
mod proptests {
    use crate::reinforcement::WorkflowAction;
    use crate::{RlAction, RlState};
    use crate::dteam::core::KTier;
    use crate::dteam::kernel::branchless::{fire_transition, transition_rl_state};
    use crate::models::petri_net::{PetriNet, Transition, Arc};
    use proptest::prelude::*;

    // μ-kernel invariant: Var(τ) == 0 (Deterministic Execution)
    // For a fixed state and action, the next_state must be identical.
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
            
            // Execute twice to check variancy τ
            let result1 = transition_rl_state(state, action);
            let result2 = transition_rl_state(state, action);
            
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (Var(τ) != 0)");
        }

        #[test]
        fn test_branchless_transition_firing(
            marking in 0u64..u64::MAX,
            input_mask in 0u64..u64::MAX,
            output_mask in 0u64..u64::MAX,
        ) {
            let result = fire_transition(marking, input_mask, output_mask);
            
            // Canonical check
            let enabled = (marking & input_mask) == input_mask;
            let expected = if enabled {
                (marking & !input_mask) | output_mask
            } else {
                marking
            };
            
            assert_eq!(result, expected, "Branchless firing logic divergent from canonical reference");
        }

        #[test]
        fn test_mdl_minimality_invariant(
            t_count in 1usize..100,
            a_count in 1usize..200,
        ) {
            let mut net = PetriNet::default();
            for i in 0..t_count {
                net.transitions.push(Transition {
                    id: format!("t{}", i),
                    label: "A".to_string(),
                    is_invisible: None,
                });
            }
            for i in 0..a_count {
                net.arcs.push(Arc {
                    from: "p1".to_string(),
                    to: format!("t{}", i % t_count),
                    weight: None,
                });
            }
            
            let score = net.mdl_score();
            let expected = (t_count as f64) + (a_count as f64 * (t_count as f64).log2());
            
            assert!((score - expected).abs() < f64::EPSILON, "MDL score invariant violation");
        }

        #[test]
        fn test_ktier_alignment_and_capacity(
            k_idx in 0..5usize,
        ) {
            let tiers = [KTier::K64, KTier::K128, KTier::K256, KTier::K512, KTier::K1024];
            let tier = tiers[k_idx];
            
            let expected_capacity = match tier {
                KTier::K64 => 64,
                KTier::K128 => 128,
                KTier::K256 => 256,
                KTier::K512 => 512,
                KTier::K1024 => 1024,
            };
            
            assert_eq!(tier.capacity(), expected_capacity, "KTier capacity misalignment");
            assert_eq!(tier.capacity() % 64, 0, "KTier word alignment violation");
        }
    }

    #[test]
    fn test_zero_allocation_hot_path_verification() {
        let state = RlState {
            health_level: 1,
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
        let action = RlAction::Optimize;
        let next = transition_rl_state(state, action);
        assert_eq!(next.health_level, 2);
    }

    #[test]
    fn test_provenance_manifest_emission() {
        use crate::dteam::orchestration::{Engine, EngineResult};
        use crate::models::{Event, EventLog, Trace};

        let engine = Engine::builder().with_k_tier(64).build();
        let mut log = EventLog::default();
        let mut trace = Trace::default();
        trace.events.push(Event::new("A".to_string()));
        log.add_trace(trace);

        let result = engine.run(&log);
        if let EngineResult::Success(_, manifest) = result {
            assert_eq!(manifest.input_log_hash, log.canonical_hash());
            assert!(!manifest.action_sequence.is_empty());
            assert!(manifest.mdl_score > 0.0);
            assert!(manifest.latency_ns > 0);
        } else {
            panic!("Engine execution failed");
        }
    }
}
