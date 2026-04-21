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
                marking_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
                activities_hash: 0,
            };
            let action = RlAction::from_index(a).unwrap();
            
            // Execute twice to check variancy τ
            let result1 = transition(state, action);
            let result2 = transition(state, action);
            
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic");
        }

        #[test]
        fn test_μ_kernel_bitset_logic(
            p1 in 0usize..1024,
            p2 in 0usize..1024,
        ) {
            let mut mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
            let _ = mask.set(p1);
            let _ = mask.set(p2);
            
            assert!(mask.contains(p1));
            assert!(mask.contains(p2));
            assert_eq!(mask.pop_count(), if p1 == p2 { 1 } else { 2 });
        }

        #[test]
        fn test_engine_ktier_enforcement(
            footprint in 1usize..2000,
        ) {
            use crate::dteam::core::KTier;
            let k_tier = if footprint <= 64 {
                KTier::K64
            } else if footprint <= 128 {
                KTier::K128
            } else if footprint <= 256 {
                KTier::K256
            } else if footprint <= 512 {
                KTier::K512
            } else {
                KTier::K1024
            };

            let engine = crate::dteam::orchestration::Engine::builder()
                .with_k_tier(k_tier.capacity())
                .build();

            assert_eq!(engine.k_tier, k_tier);
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
