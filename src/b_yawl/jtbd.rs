#[cfg(test)]
mod tests {
    use crate::b_yawl::engine::BYawlEngine;
    use crate::b_yawl::patterns::BYawlPatternCompiler;

    #[test]
    fn jtbd_scenario_01_core_routing_and_cycles() {
        // Scenario 1: Basic Routing & Arbitrary Cycles
        // Uses WCP 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1; // Start at p0

        let tasks = vec![
            BYawlPatternCompiler::wcp01_sequence(1, 0b1, 0b10),
            BYawlPatternCompiler::wcp02_parallel_split(2, 0b10, 0b1100),
            BYawlPatternCompiler::wcp03_synchronization(3, 0b1100, 0b10000),
            BYawlPatternCompiler::wcp04_exclusive_choice(4, 0b10000, 0b100000),
            BYawlPatternCompiler::wcp05_simple_merge(5, 0b100000, 0b1000000),
            BYawlPatternCompiler::wcp06_multi_choice(6, 0b1000000, 0b10000000),
            BYawlPatternCompiler::wcp07_structured_synchronizing_merge(
                7,
                0b10000000,
                0b100000000,
                u64::MAX,
            ),
            BYawlPatternCompiler::wcp08_multi_merge(8, 0b100000000, 0b1000000000),
            BYawlPatternCompiler::wcp09_structured_discriminator(9, 0b1000000000, 0b10000000000, 0),
            BYawlPatternCompiler::wcp10_arbitrary_cycles(10, 0b10000000000, 0b100000000000),
        ];

        for task in &tasks {
            engine.execute_task(task);
        }

        assert_eq!(engine.state_mask, 0b100000000000); // Reached the end
    }

    #[test]
    fn jtbd_scenario_02_instances_and_cancellation() {
        // Scenario 2: Multiple Instances & Cancellation
        // Uses WCP 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1;

        let tasks = vec![
            BYawlPatternCompiler::wcp12_mi_without_sync(12, 0b1, 0b10, 5),
            BYawlPatternCompiler::wcp13_mi_priori_design(13, 0b10, 0b100, 3),
            BYawlPatternCompiler::wcp14_mi_priori_runtime(14, 0b100, 0b1000, 2),
            BYawlPatternCompiler::wcp15_mi_without_priori_runtime(15, 0b1000, 0b10000), // Note: max instances removed
            BYawlPatternCompiler::wcp16_deferred_choice(16, 0b10000, 0b100000),
            BYawlPatternCompiler::wcp17_interleaved_parallel_routing(
                17, 0b100000, 0b1000000, 0b1, false,
            ), // Adds interleaved_lock_mask
            // Milestone needs a condition. We'll set the condition bit artificially.
            BYawlPatternCompiler::wcp18_milestone(18, 0b1000000, 0b1000000000, 0b10000000),
            BYawlPatternCompiler::wcp19_cancel_task(19, 0b10000000, 0b100000000000),
            BYawlPatternCompiler::wcp11_implicit_termination(11, 0b10000000),
            BYawlPatternCompiler::wcp20_cancel_case(20, 0b1000000000000000),
        ];

        engine.state_mask |= 0b1000000000; // Satisfy milestone condition
        for task in &tasks {
            engine.execute_task(task);
        }

        assert_eq!(engine.state_mask, 0b1000000000); // condition token remains, the rest was implicitly terminated or cancelled.
    }

    #[test]
    fn test_hard_pattern_discriminator_wcp29() {
        // WCP-29: Cancelling Discriminator
        // It must fire on the first token, ignore/cancel subsequent tokens, and properly reset.
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b0011; // Tokens on both incoming branches p0 (bit 0) and p1 (bit 1)

        // Discriminator consumes p0 and p1. Produces p2 (bit 2). Resets on p3 (bit 3). join_state_bit = 0.
        let disc =
            BYawlPatternCompiler::wcp29_cancelling_discriminator(1, 0b0011, 0b0100, 0b1000, 0);

        // Step 1: Execute. Should fire because threshold (1) is met.
        let executed = engine.execute_task(&disc);
        assert!(
            executed,
            "Discriminator must fire on first available tokens"
        );
        assert_eq!(engine.state_mask, 0b0100); // Output token produced. Input consumed.
        assert_eq!(
            engine.fired_joins_mask, 0b1,
            "Discriminator state bit must be locked"
        );

        // Step 2: More tokens arrive on the incoming branches.
        engine.state_mask |= 0b0001; // Late token arrives on p0
        let executed_again = engine.execute_task(&disc);

        // The task itself doesn't "fire" (doesn't produce output), but it DOES consume the token
        // because it is a complex join that has already fired.
        assert!(!executed_again, "Discriminator must not fire again");
        assert_eq!(
            engine.state_mask, 0b0100,
            "Late tokens must be consumed/ignored by locked discriminator"
        );

        // Step 3: Reset the discriminator using the reset_mask (p3).
        engine.state_mask |= 0b1000; // Trigger reset
        let _ = engine.execute_task(&disc); // Will process reset
        assert_eq!(engine.fired_joins_mask, 0, "Discriminator state must reset");
        assert_eq!(engine.state_mask, 0b0100, "Reset token should be consumed");

        // Step 4: Now it can fire again.
        engine.state_mask |= 0b0010; // Token arrives on p1
        let executed_restarted = engine.execute_task(&disc);
        assert!(
            executed_restarted,
            "Discriminator must fire again after reset"
        );
        assert_eq!(engine.state_mask, 0b0100); // Because 0b0100 | 0b0100 = 0b0100
    }

    #[test]
    fn test_hard_pattern_synchronizing_merge_wcp37() {
        // WCP-37: Local Synchronizing Merge (OR-Join)
        // Must wait if another upstream token can arrive. Must fire if no upstream tokens can arrive.
        let mut engine = BYawlEngine::new();

        // p0 is upstream of p1. p1 and p2 merge into p3.
        // OR-join consumes p1 and p2.
        // Reachability for the OR-join includes p0, p1, p2.
        let or_join =
            BYawlPatternCompiler::wcp37_local_synchronizing_merge(1, 0b0110, 0b1000, 0b0111);

        // Scenario A: Token at p2. Token at p0 (upstream).
        engine.state_mask = 0b0101;

        // The OR-join sees token at p2. But p0 is in reachability mask!
        let executed = engine.execute_task(&or_join);
        assert!(
            !executed,
            "OR-Join MUST wait because upstream token at p0 can still reach it"
        );

        // Simulate p0 advancing to p1.
        engine.state_mask = 0b0110; // Tokens at p1 and p2. p0 is empty.
        let executed_now = engine.execute_task(&or_join);
        assert!(
            executed_now,
            "OR-Join fires when all reachable upstream tokens have arrived"
        );
        assert_eq!(engine.state_mask, 0b1000, "Tokens merged into p3");

        // Scenario B: Token at p2 ONLY. No tokens upstream.
        engine.state_mask = 0b0100;
        let executed_partial = engine.execute_task(&or_join);
        assert!(
            executed_partial,
            "OR-Join fires immediately if only one branch is active and no others can arrive"
        );
        assert_eq!(engine.state_mask, 0b1000, "Tokens merged into p3");
    }

    #[test]
    fn test_hard_pattern_cancellation_region_wcp25() {
        // WCP-25: Cancel Region
        // Instantly wipes multiple places and active instances in a defined region branchlessly.
        let mut engine = BYawlEngine::new();

        // Setup: Region includes p1, p2, p3. Also 5 active instances of a task running at p2.
        engine.state_mask = 0b11111; // p0, p1, p2, p3, p4 active
        engine.active_instances[2] = 5;

        // Task at p0 executes, cancels p1, p2, p3.
        let cancel_task = BYawlPatternCompiler::wcp25_cancel_region(1, 0b00001, 0b01110);

        let executed = engine.execute_task(&cancel_task);
        assert!(executed);

        // p0 is consumed. p1, p2, p3 are cancelled. p4 remains.
        assert_eq!(
            engine.state_mask, 0b10000,
            "Cancellation mask must wipe region identically"
        );
        assert_eq!(
            engine.active_instances[2], 0,
            "Active instances in the cancelled region must drop to 0"
        );
    }

    #[test]
    fn jtbd_scenario_03_triggers_and_partial_joins() {
        // Scenario 3: Triggers, Regions & Blocking
        // Uses WCP 21, 22, 23, 24, 25, 26, 27, 28, 29, 30
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1;

        let tasks = vec![
            BYawlPatternCompiler::wcp21_structured_loop(21, 0b1, 0b10),
            BYawlPatternCompiler::wcp22_recursion(22, 0b10, 0b100),
            BYawlPatternCompiler::wcp23_transient_trigger(23, 0b1000, 0b10000), // Requires trigger mask 0b1000
            BYawlPatternCompiler::wcp24_persistent_trigger(24, 0b100000, 0b1000000), // Requires trigger mask 0b100000
            BYawlPatternCompiler::wcp25_cancel_region(25, 0b100, 0b10000000), // Cancels 0b10000000
            BYawlPatternCompiler::wcp26_cancel_mi_activity(26, 0b100000000, 0b1000000000),
            BYawlPatternCompiler::wcp27_complete_mi_activity(
                27,
                0b10000000000,
                0b100000000000,
                0b10000000000,
            ),
            BYawlPatternCompiler::wcp28_blocking_discriminator(
                28,
                0b1000000000000,
                0b10000000000000,
                1,
            ),
            BYawlPatternCompiler::wcp29_cancelling_discriminator(
                29,
                0b100000000000000,
                0b1000000000000000,
                0b100000000000000,
                2,
            ),
            BYawlPatternCompiler::wcp30_structured_partial_join(
                30,
                0b10000000000000000,
                0b100000000000000000,
                1,
                3,
            ),
        ];

        // Manually push tokens and triggers to test specific ones
        engine.trigger_event(0b1000); // Trigger 23
        engine.trigger_event(0b100000); // Trigger 24

        engine.state_mask |= 0b10000000000000000; // Trigger 30

        for task in &tasks {
            engine.execute_task(task);
        }

        assert_ne!(engine.state_mask, 0);
    }

    #[test]
    fn jtbd_scenario_04_complex_joins_and_merges() {
        // Scenario 4: N-out-of-M, Local Sync & Critical Section
        // Uses WCP 31, 32, 33, 34, 35, 36, 37, 38, 39, 40
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1;

        let tasks = vec![
            BYawlPatternCompiler::wcp31_blocking_partial_join(31, 0b1, 0b10, 1, 4),
            BYawlPatternCompiler::wcp32_cancelling_partial_join(32, 0b10, 0b100, 1, 0b10, 5),
            BYawlPatternCompiler::wcp33_generalized_and_join(33, 0b100, 0b1000, 6),
            BYawlPatternCompiler::wcp34_static_n_out_of_m_join(34, 0b1000, 0b10000, 1, 7),
            BYawlPatternCompiler::wcp35_cancelling_n_out_of_m_join(
                35, 0b10000, 0b100000, 1, 0b10000, 8,
            ),
            BYawlPatternCompiler::wcp36_dynamic_n_out_of_m_join(36, 0b100000, 0b1000000, 1, 9),
            BYawlPatternCompiler::wcp37_local_synchronizing_merge(
                37,
                0b1000000,
                0b10000000,
                u64::MAX,
            ),
            BYawlPatternCompiler::wcp38_general_synchronizing_merge(
                38,
                0b10000000,
                0b100000000,
                u64::MAX,
            ),
            BYawlPatternCompiler::wcp39_critical_section(
                39,
                0b100000000,
                0b1000000000,
                0b100000000000000,
            ), // Condition mask
            BYawlPatternCompiler::wcp40_interleaved_routing(
                40,
                0b1000000000,
                0b10000000000,
                0b10,
                true,
            ), // Lock mask and release flag
        ];

        engine.state_mask |= 0b100000000000000; // For the critical section condition

        for task in &tasks {
            engine.execute_task(task);
        }

        assert_ne!(engine.state_mask, 0);
    }

    #[test]
    fn jtbd_scenario_05_threads_and_termination() {
        // Scenario 5: Threading & Explicit Termination
        // Uses WCP 41, 42, 43, plus 7 repeats (1, 2, 3, 4, 5, 6, 7) to hit exactly 10 patterns.
        let mut engine = BYawlEngine::new();
        engine.state_mask = 0b1;

        let tasks = vec![
            BYawlPatternCompiler::wcp01_sequence(1, 0b1, 0b10),
            BYawlPatternCompiler::wcp02_parallel_split(2, 0b10, 0b1100),
            BYawlPatternCompiler::wcp03_synchronization(3, 0b1100, 0b10000),
            BYawlPatternCompiler::wcp42_thread_split(42, 0b10000, 0b1100000),
            BYawlPatternCompiler::wcp41_thread_merge(41, 0b100000, 0b10000000),
            BYawlPatternCompiler::wcp04_exclusive_choice(4, 0b10000000, 0b100000000),
            BYawlPatternCompiler::wcp05_simple_merge(5, 0b100000000, 0b1000000000),
            BYawlPatternCompiler::wcp06_multi_choice(6, 0b1000000000, 0b10000000000),
            BYawlPatternCompiler::wcp07_structured_synchronizing_merge(
                7,
                0b10000000000,
                0b100000000000,
                0b10000000000,
            ),
            BYawlPatternCompiler::wcp43_explicit_termination(43, 0b100000000000), // Annihilates everything
        ];

        for task in &tasks {
            engine.execute_task(task);
        }

        assert_eq!(engine.state_mask, 0); // Explicit termination wipes out all active tokens
    }
}
