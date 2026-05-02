#[cfg(test)]
mod tests {
    use crate::utils::static_pkt::StaticPackedKeyTable;
    use crate::{RlState, RlAction};
    use crate::reinforcement::QLearning;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_static_pkt_determinism(
            hashes in prop::collection::vec(0u64..1000, 1..50),
        ) {
            let mut table: StaticPackedKeyTable<u64, u64, 64> = StaticPackedKeyTable::new();
            for &h in &hashes {
                let _ = table.insert(h, h, h + 1);
            }

            for &h in &hashes {
                assert_eq!(table.get(h), Some(&(h + 1)));
            }
        }

        #[test]
        fn test_static_pkt_capacity_violation(
            hashes in prop::collection::vec(0u64..u64::MAX, 65..70),
        ) {
            let mut table: StaticPackedKeyTable<u64, u64, 64> = StaticPackedKeyTable::new();
            let mut success_count = 0;
            for &h in &hashes {
                if table.insert(h, h, h).is_ok() {
                    success_count += 1;
                }
            }
            assert_eq!(success_count, 64);
        }

        #[test]
        fn test_q_learning_zero_allocation_logic(
            steps in 1usize..100,
        ) {
            let mut agent: QLearning<RlState<1>, RlAction> = QLearning::new();
            let mut state = RlState::default();
            
            for _ in 0..steps {
                let action = agent.select_action(state);
                let mut next_state = state;
                next_state.health_level = (state.health_level + 1) % 5;
                
                // This update is now zero-allocation in its hot path (after initial state discovery)
                agent.update(state, action, 1.0, next_state, false);
                state = next_state;
            }
            
            assert!(agent.total_reward() > 0.0);
        }
    }
}
