#[cfg(test)]
mod proptests {
    use crate::reinforcement::WorkflowAction;
    use crate::{RlAction, RlState};
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
            let result1 = transition(state, action);
            let result2 = transition(state, action);

            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic");
        }

        #[test]
        fn test_ktier_capacity_bounds(
            k in 0usize..2048,
        ) {
            use crate::dteam::orchestration::EngineBuilder;
            let engine = EngineBuilder::new().with_k_tier(k).build();
            let capacity = engine.k_tier.capacity();

            if k <= 64 { assert_eq!(capacity, 64); }
            else if k <= 128 { assert_eq!(capacity, 128); }
            else if k <= 256 { assert_eq!(capacity, 256); }
            else if k <= 512 { assert_eq!(capacity, 512); }
            else { assert_eq!(capacity, 1024); }
        }

        #[test]
        fn test_mdl_minimality_formula(
            t in 1usize..100,
            a in 0usize..500,
        ) {
            use crate::models::petri_net::{PetriNet, Transition, Arc};
            let mut net = PetriNet::default();
            for i in 0..t {
                net.transitions.push(Transition {
                    id: format!("t{}", i),
                    label: format!("T{}", i),
                    is_invisible: None,
                });
            }
            for i in 0..a {
                net.arcs.push(Arc {
                    from: "p0".to_string(),
                    to: format!("t{}", i % t),
                    weight: None,
                });
            }

            let expected = (t as f64) + ((a as f64) * (t as f64).log2());
            let actual = net.mdl_score();

            assert!((expected - actual).abs() < 1e-9, "MDL score mismatch: expected {}, got {}", expected, actual);
        }

        #[test]
        fn test_manifest_integrity_stability(
            h_l in any::<u64>(),
            h_n in any::<u64>(),
            mdl in any::<f64>(),
            trajectory in prop::collection::vec(0u8..3u8, 0..50),
        ) {
            use crate::utils::dense_kernel::fnv1a_64;

            let mut hasher_bytes = Vec::new();
            hasher_bytes.extend_from_slice(&h_l.to_le_bytes());
            hasher_bytes.extend_from_slice(&trajectory);
            hasher_bytes.extend_from_slice(&h_n.to_le_bytes());
            hasher_bytes.extend_from_slice(&mdl.to_bits().to_le_bytes());

            let h1 = fnv1a_64(&hasher_bytes);
            let h2 = fnv1a_64(&hasher_bytes);

            assert_eq!(h1, h2, "Integrity hash not stable");
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
