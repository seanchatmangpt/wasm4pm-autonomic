//! Encoding of the 43 Workflow Patterns (van der Aalst et al.) into Binary YAWL (bYAWL).
//! Maps high-level semantics into nanosecond-ready 64-byte structs.

use super::format::*;

pub struct BYawlPatternCompiler;

impl BYawlPatternCompiler {
    fn base_task(id: u16) -> BYawlTask {
        BYawlTask {
            id,
            join_type: JoinType::AND,
            split_type: SplitType::AND,
            min_instances: 1,
            max_instances: 1,
            threshold_instances: 1,
            flags: 0,
            join_state_bit: 0,
            consume_mask: 0,
            produce_mask: 0,
            cancellation_mask: 0,
            condition_mask: 0,
            reset_mask: 0,
            reachability_mask: u64::MAX, // Default: assumes anything can reach
            interleaved_lock_mask: 0,
        }
    }

    // 1. Basic Control Flow
    pub fn wcp01_sequence(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            ..Self::base_task(id)
        }
    }
    pub fn wcp02_parallel_split(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::AND,
            ..Self::base_task(id)
        }
    }
    pub fn wcp03_synchronization(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::AND,
            ..Self::base_task(id)
        }
    }
    pub fn wcp04_exclusive_choice(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::XOR,
            ..Self::base_task(id)
        }
    }
    pub fn wcp05_simple_merge(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::XOR,
            ..Self::base_task(id)
        }
    }

    // 2. Advanced Branching and Synchronization
    pub fn wcp06_multi_choice(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::OR,
            ..Self::base_task(id)
        }
    }
    pub fn wcp07_structured_synchronizing_merge(
        id: u16,
        consume: u64,
        produce: u64,
        reachability_mask: u64,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::OR,
            reachability_mask,
            ..Self::base_task(id)
        }
    }
    pub fn wcp08_multi_merge(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::XOR,
            split_type: SplitType::AND,
            ..Self::base_task(id)
        }
    }
    pub fn wcp09_structured_discriminator(
        id: u16,
        consume: u64,
        produce: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: 1,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp10_arbitrary_cycles(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::XOR,
            split_type: SplitType::XOR,
            ..Self::base_task(id)
        }
    }
    pub fn wcp11_implicit_termination(id: u16, consume: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            split_type: SplitType::ImplicitTermination,
            ..Self::base_task(id)
        }
    }

    // 3. Multiple Instance Patterns
    pub fn wcp12_mi_without_sync(id: u16, consume: u64, produce: u64, max: u8) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::MultiInstance,
            max_instances: max,
            ..Self::base_task(id)
        }
    }
    pub fn wcp13_mi_priori_design(id: u16, consume: u64, produce: u64, max: u8) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::MultiInstance,
            max_instances: max,
            ..Self::base_task(id)
        }
    }
    pub fn wcp14_mi_priori_runtime(id: u16, consume: u64, produce: u64, max: u8) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::MultiInstance,
            max_instances: max,
            ..Self::base_task(id)
        }
    }
    pub fn wcp15_mi_without_priori_runtime(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::DynamicMultiInstance,
            ..Self::base_task(id)
        }
    }

    // 4. State-based Patterns
    pub fn wcp16_deferred_choice(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::DeferredChoice,
            ..Self::base_task(id)
        }
    }
    pub fn wcp17_interleaved_parallel_routing(
        id: u16,
        consume: u64,
        produce: u64,
        lock_mask: u64,
        release_lock: bool,
    ) -> BYawlTask {
        let mut flags = 0;
        if release_lock {
            flags |= 4;
        }
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::InterleavedRouting,
            interleaved_lock_mask: lock_mask,
            flags,
            ..Self::base_task(id)
        }
    }
    pub fn wcp18_milestone(id: u16, consume: u64, condition: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            condition_mask: condition,
            ..Self::base_task(id)
        }
    }

    // 5. Cancellation and Force Completion Patterns
    pub fn wcp19_cancel_task(id: u16, consume: u64, cancel: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            cancellation_mask: cancel,
            ..Self::base_task(id)
        }
    }
    pub fn wcp20_cancel_case(id: u16, consume: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            cancellation_mask: u64::MAX,
            ..Self::base_task(id)
        }
    }
    pub fn wcp21_structured_loop(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::XOR,
            ..Self::base_task(id)
        }
    }
    pub fn wcp22_recursion(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::XOR,
            split_type: SplitType::XOR,
            ..Self::base_task(id)
        }
    }
    pub fn wcp23_transient_trigger(id: u16, trigger_mask: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: trigger_mask,
            produce_mask: produce,
            flags: 1,
            ..Self::base_task(id)
        }
    }
    pub fn wcp24_persistent_trigger(id: u16, trigger_mask: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: trigger_mask,
            produce_mask: produce,
            flags: 2,
            ..Self::base_task(id)
        }
    }
    pub fn wcp25_cancel_region(id: u16, consume: u64, cancel: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            cancellation_mask: cancel,
            ..Self::base_task(id)
        }
    }
    pub fn wcp26_cancel_mi_activity(id: u16, consume: u64, cancel: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            cancellation_mask: cancel,
            ..Self::base_task(id)
        }
    }
    pub fn wcp27_complete_mi_activity(
        id: u16,
        consume: u64,
        produce: u64,
        cancel_instances_mask: u64,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            cancellation_mask: cancel_instances_mask,
            flags: 8,
            ..Self::base_task(id)
        }
    }

    // Extensions
    pub fn wcp28_blocking_discriminator(
        id: u16,
        consume: u64,
        produce: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: 1,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp29_cancelling_discriminator(
        id: u16,
        consume: u64,
        produce: u64,
        reset: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: 1,
            reset_mask: reset,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp30_structured_partial_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp31_blocking_partial_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp32_cancelling_partial_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        reset: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            reset_mask: reset,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp33_generalized_and_join(
        id: u16,
        consume: u64,
        produce: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: consume.count_ones() as u8,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp34_static_n_out_of_m_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp35_cancelling_n_out_of_m_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        reset: u64,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            reset_mask: reset,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp36_dynamic_n_out_of_m_join(
        id: u16,
        consume: u64,
        produce: u64,
        threshold: u8,
        join_state_bit: u8,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::Complex,
            threshold_instances: threshold,
            join_state_bit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp37_local_synchronizing_merge(
        id: u16,
        consume: u64,
        produce: u64,
        reachability_mask: u64,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::OR,
            reachability_mask,
            ..Self::base_task(id)
        }
    }
    pub fn wcp38_general_synchronizing_merge(
        id: u16,
        consume: u64,
        produce: u64,
        reachability_mask: u64,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::OR,
            reachability_mask,
            ..Self::base_task(id)
        }
    }
    pub fn wcp39_critical_section(
        id: u16,
        consume: u64,
        produce: u64,
        condition: u64,
    ) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            condition_mask: condition,
            ..Self::base_task(id)
        }
    }
    pub fn wcp40_interleaved_routing(
        id: u16,
        consume: u64,
        produce: u64,
        lock_mask: u64,
        release_lock: bool,
    ) -> BYawlTask {
        let mut flags = 0;
        if release_lock {
            flags |= 4;
        }
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::InterleavedRouting,
            interleaved_lock_mask: lock_mask,
            flags,
            ..Self::base_task(id)
        }
    }
    pub fn wcp41_thread_merge(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            join_type: JoinType::ThreadMerge,
            ..Self::base_task(id)
        }
    }
    pub fn wcp42_thread_split(id: u16, consume: u64, produce: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            produce_mask: produce,
            split_type: SplitType::ThreadSplit,
            ..Self::base_task(id)
        }
    }
    pub fn wcp43_explicit_termination(id: u16, consume: u64) -> BYawlTask {
        BYawlTask {
            consume_mask: consume,
            split_type: SplitType::ExplicitTermination,
            ..Self::base_task(id)
        }
    }
}
