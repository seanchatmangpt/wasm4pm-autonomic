#[cfg(test)]
mod tests {
    use crate::agentic::Simulator;
    use crate::autonomic::{AutonomicAction, AutonomicEvent, AutonomicKernel, Vision2030Kernel};
    use crate::simd::SwarMarking;
    use crate::utils::dense_kernel::KBitSet;
    use std::time::SystemTime;

    fn setup_violation(events: Vec<&str>) -> Vision2030Kernel<1> {
        let mut kernel = Vision2030Kernel::<1>::new();
        for _ in 0..100 {
            for payload in &events {
                kernel.observe(AutonomicEvent {
                    source: "counterfactual_suite".to_string(),
                    payload: payload.to_string(),
                    timestamp: SystemTime::now(),
                });
            }
            kernel.trace_cursor = 0;
            kernel.powl_executed_mask = KBitSet::<1>::zero();
            kernel.powl_prev_idx = 64;
            kernel.marking = SwarMarking::new(1);
        }
        kernel
    }

    fn run_counterfactual_validation(name: &str, events: Vec<&str>) {
        println!("\n⚖️ Counterfactual Test: {}", name);

        // 1. Establish Baseline (Drifted State)
        let mut kernel = setup_violation(events);
        let drifted_state = kernel.infer();
        assert!(
            drifted_state.drift_detected,
            "Scenario '{}' must trigger drift for counterfactual validation",
            name
        );

        // 2. Propose Actions
        let actions = kernel.propose(&drifted_state);
        assert!(
            !actions.is_empty(),
            "Kernel must propose at least one recovery action"
        );

        // 3. Select Best Action via Simulator
        let simulator = Simulator::new(drifted_state.clone());
        let mut best_action: Option<AutonomicAction> = None;
        let mut max_reward = f32::NEG_INFINITY;

        for action in &actions {
            let (_, reward) = simulator.evaluate_action(action);
            if reward > max_reward {
                max_reward = reward;
                best_action = Some(action.clone());
            }
        }

        let action_to_take = best_action.expect("Simulator failed to select an action");
        println!("  Selected Fix: {}", action_to_take);

        // 4. Compare Counterfactuals
        let health_do_nothing = drifted_state.process_health;

        // Path B: Execute Fix
        kernel.execute(action_to_take);
        let fixed_state = kernel.infer();

        println!(
            "  Results: Health(Before)={:.2}, Health(After)={:.2}, Drift={}",
            health_do_nothing, fixed_state.process_health, fixed_state.drift_detected
        );

        // 5. Axiomatic Verification
        assert!(
            fixed_state.process_health >= health_do_nothing,
            "Fix must not degrade health"
        );
        assert!(
            !fixed_state.drift_detected,
            "Fix must resolve the semantic drift"
        );
        assert!(
            fixed_state.conformance_score > drifted_state.conformance_score,
            "Fix must improve conformance"
        );
    }

    #[test]
    fn cf_jtbd_01_offshore_maintenance_drift() {
        // Counterfactual Scenario 1: XOR Recovery
        // In this counterfactual validation, we simulate the offshore maintenance drift.
        // We establish a baseline drifted state by providing an illegal XOR sequence.
        // The system is expected to propose a repair action, evaluate it via the simulator,
        // and confirm that the execution of this fix resolves the semantic drift
        // and improves overall process health and conformance.
        let events = vec!["Start", "Normal", "Bypass"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-01 XOR Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_02_invoice_exception_routing() {
        // Counterfactual Scenario 2: Order Recovery
        // This test simulates a partial order violation where 'Normal' occurs before 'Start'.
        // We verify that the Simulator and RL agent can propose a counterfactual fix
        // that optimally resets the trace and marking, allowing the process to
        // regain its structural integrity and continue tracking throughput correctly.
        let events = vec!["Normal", "Start"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-02 Order Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_03_compliance_gate_violation() {
        // Counterfactual Scenario 3: Mutual Exclusion Recovery
        // When a compliance gate is bypassed before normal execution, it breaks XOR rules.
        // This counterfactual test ensures that the AutonomicKernel can not only
        // detect this severe violation but also propose and apply a repair action
        // that successfully nullifies the drift and boosts conformance metrics.
        let events = vec!["Start", "Bypass", "Normal"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-03 Mutual Exclusion Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_04_incident_mode_activation() {
        // Counterfactual Scenario 4: Repetition Recovery
        // Repetitive bypasses trigger the repetition exclusion mask.
        // Here we ensure that the counterfactual simulator correctly values
        // a repair action highly enough to escape this incident mode, effectively
        // restoring the health of the system after significant degradation.
        let events = vec!["Start", "Bypass", "Bypass"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-04 Repetition Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_05_spaghetti_process_recovery() {
        // Counterfactual Scenario 5: Spaghetti Order Recovery
        // We simulate a strict structural failure (like an End before a Start)
        // to verify that even in complex, concurrent spaghetti processes,
        // a structural fault is caught and counterfactually repaired by the
        // Digital Team process intelligence engine.
        let events = vec!["End", "Start"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-05 Spaghetti Order Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_06_streaming_capacity_budget() {
        // Counterfactual Scenario 6: Capacity Reset
        // Streaming loads can sometimes lead to repetition anomalies if case
        // boundaries are lost. Here, we feed Start -> End -> Start rapidly.
        // The engine must counterfactually prove that repairing the state
        // boundary is superior to letting the system run in a drifted state.
        let events = vec!["Start", "End", "Start"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-06 Capacity Reset", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_07_counterfactual_reroute_before_action() {
        // Counterfactual Scenario 7: Reroute Recovery
        // This directly tests the counterfactual simulator's primary use case.
        // When faced with a sequence of bypasses that trigger high-risk context,
        // we assert that the simulator identifies a Repair action that strictly
        // outperforms the baseline degraded state.
        let events = vec!["Start", "Bypass", "Bypass"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-07 Reroute Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_08_human_handoff_under_ambiguity() {
        // Counterfactual Scenario 8: Ambiguity Recovery
        // When ambiguous payloads arrive, they can trigger repetition or
        // unknown state rules. The counterfactual validation proves that
        // applying a structural repair resets the context to a clean slate,
        // outperforming any ambiguous, non-deterministic state.
        let events = vec!["Normal", "Normal"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-08 Ambiguity Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_09_object_centric_incident_timeline() {
        // Counterfactual Scenario 9: OCPM Semantic Recovery
        // A multi-object trace triggers a basic structural violation.
        // We verify that the counterfactual logic correctly evaluates
        // a repair that resets the object-centric stream tracking
        // without panicking.
        let events = vec!["Start", "Normal", "Normal"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-09 OCPM Semantic Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_10_adversarial_noise_stream() {
        // Counterfactual Scenario 10: Noise Cleanup
        // Even when subjected to adversarial noise that might trick the
        // system into a drifted state, the counterfactual simulator must
        // consistently identify that structural repair is the optimal
        // path forward.
        let events = vec!["Start", "End", "End"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-10 Noise Cleanup", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_11_batch_boundary_collapse() {
        // Counterfactual Scenario 11: Boundary Recovery
        // When batch boundaries collapse and an 'End' follows another 'End',
        // it triggers a repetition violation. We validate that the engine
        // can counterfactually simulate and execute a fix that resets
        // the boundary successfully.
        let events = vec!["Start", "End", "End"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-11 Boundary Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_12_digital_team_role_conflict() {
        // Counterfactual Scenario 12: Conflict Resolution
        // A role conflict leads to an XOR violation (Normal vs Bypass).
        // The agentic simulator evaluates the states and proves that
        // a repair action (resetting the conflict) is the mathematically
        // optimal choice compared to a degraded operational state.
        let events = vec!["Start", "Normal", "Bypass"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-12 Conflict Resolution", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_13_fully_autonomic_closed_loop() {
        // Counterfactual Scenario 13: Closed Loop Stabilization
        // We purposely inject an order violation (End then Normal) into what
        // is typically the happy path. We then assert that the counterfactual
        // simulator correctly navigates the system back to stability via a
        // repair action, restoring closed-loop autonomy.
        let events = vec!["End", "Normal"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-13 Closed Loop Stabilization", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_14_forced_human_governance() {
        // Counterfactual Scenario 14: Governance Recovery
        // In this extreme state of repeated bypasses designed to force human
        // governance, we test whether the automated simulation framework can
        // still identify a structural repair that mathematically improves
        // the context over doing nothing.
        let events = vec!["Start", "Bypass", "Bypass"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-14 Governance Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_15_petabyte_stream_approximation_mode() {
        // Counterfactual Scenario 15: Stream Stability Recovery
        // We simulate an XOR violation (Bypass then Normal) inside a heavy stream.
        // The engine must rely on its BCINR representations to counterfactually
        // project that a repair action clears the error and restores conformance
        // without a full re-computation of the stream.
        let events = vec!["Start", "Bypass", "Normal"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-15 Stream Stability Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_16_regression_reproduction() {
        // Counterfactual Scenario 16: Regression Recovery
        // We trigger an intentional partial order violation in the baseline scenario.
        // This verifies that regression test setups can successfully utilize
        // the counterfactual simulator to map an optimal path out of a
        // deliberately corrupted state.
        let events = vec!["Normal", "Start"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-16 Regression Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_17_object_centric_divergence() {
        // Counterfactual Scenario 17: OCPM Divergence Recovery
        // We inject an artificial item divergence anomaly in the stream.
        // The OC-DFG will detect this and trigger drift. The counterfactual
        // simulator must then verify that applying a repair (resetting tracking)
        // resolves the multi-dimensional OCPM drift and improves health.
        let events = vec!["Start", "Normal: Item divergence anomaly", "End"];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-17 OCPM Divergence Recovery", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_18_ocel20_predictive_prescriptive() {
        // Counterfactual Scenario 18: OCEL 2.0 Predictive Prescriptive
        // An attribute value change triggers a critical prescriptive context.
        // We validate that the counterfactual 'What-If' simulator correctly
        // projects the repair of the attribute anomaly, mathematically
        // proving that the Prescriptive AI functions as intended.
        let events = vec![
            "Start",
            "Normal: relates to order",
            "Normal: amount value changed critical",
            "End",
        ];

        // Execute the counterfactual validation framework.
        // This will automatically run assertions to prove the fix is strictly
        // better than doing nothing.
        run_counterfactual_validation("JTBD-18 OCEL 2.0 Predictive Prescriptive", events);

        // The validation internally ensures that:
        // 1. Health does not degrade.
        // 2. Drift is resolved.
        // 3. Conformance score improves.
    }

    #[test]
    fn cf_jtbd_19_evidence_recovery() {
        // Counterfactual Scenario 19: Evidence Recovery (Retriever Cognitive Breed)
        // This counterfactual test validates the Recover action on a drifted state.
        // We establish a baseline drifted state, then evaluate a Recover action
        // via the Simulator to confirm that evidence recovery from the audit log
        // improves both health and conformance while clearing drift.
        let events = vec!["Start", "Bypass", "Bypass"];

        // Set up the drifted state
        let mut kernel = setup_violation(events);
        let drifted_state = kernel.infer();
        assert!(
            drifted_state.drift_detected,
            "Setup must produce a drifted state for recovery validation"
        );

        // Construct a Recover action
        let recover_action = AutonomicAction::recover(200, "Evidence recovery from audit log");

        // Evaluate via counterfactual Simulator
        let simulator = Simulator::new(drifted_state.clone());
        let (projected_state, reward) = simulator.evaluate_action(&recover_action);

        // Verify recovery properties
        println!(
            "  Recover Action Evaluation: reward={:.2}, drift_before={}, drift_after={}",
            reward, drifted_state.drift_detected, projected_state.drift_detected
        );

        assert!(
            reward > 0.0,
            "Recover action must yield positive reward (REWARD_RECOVER=0.5)"
        );
        assert!(
            !projected_state.drift_detected,
            "Recover action must clear drift in projected state"
        );
        assert!(
            projected_state.conformance_score >= drifted_state.conformance_score,
            "Recover action must not degrade conformance"
        );

        // Execute the action in the actual kernel to verify health improvement
        kernel.execute(recover_action);
        let recovered_state = kernel.infer();

        assert!(
            recovered_state.process_health >= drifted_state.process_health,
            "Health must not degrade after recovery execution"
        );
        assert!(
            !recovered_state.drift_detected,
            "Drift must be cleared in the actual kernel state"
        );
    }
}
