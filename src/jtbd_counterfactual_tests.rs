#[cfg(test)]
mod tests {
    use crate::autonomic::{AutonomicKernel, Vision2030Kernel, AutonomicEvent, AutonomicAction, ActionType};
    use crate::agentic::Simulator;
    use std::time::SystemTime;

    fn setup_violation(events: Vec<&str>) -> Vision2030Kernel {
        let mut kernel = Vision2030Kernel::new();
        for payload in events {
            kernel.observe(AutonomicEvent {
                source: "counterfactual_suite".to_string(),
                payload: payload.to_string(),
                timestamp: SystemTime::now(),
            });
        }
        kernel
    }

    fn run_counterfactual_validation(name: &str, events: Vec<&str>) {
        println!("\n⚖️ Counterfactual Test: {}", name);
        
        // 1. Establish Baseline (Drifted State)
        let mut kernel = setup_violation(events);
        let drifted_state = kernel.infer();
        assert!(drifted_state.drift_detected, "Scenario '{}' must trigger drift for counterfactual validation", name);

        // 2. Propose Actions
        let actions = kernel.propose(&drifted_state);
        assert!(!actions.is_empty(), "Kernel must propose at least one recovery action");

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

        println!("  Results: Health(Before)={:.2}, Health(After)={:.2}, Drift={}", 
            health_do_nothing, fixed_state.process_health, fixed_state.drift_detected);

        // 5. Axiomatic Verification
        assert!(fixed_state.process_health >= health_do_nothing, "Fix must not degrade health");
        assert!(!fixed_state.drift_detected, "Fix must resolve the semantic drift");
        assert!(fixed_state.conformance_score > drifted_state.conformance_score, "Fix must improve conformance");
    }

    #[test] fn cf_jtbd_01_offshore_maintenance_drift() {
        run_counterfactual_validation("JTBD-01 XOR Recovery", 
            vec!["Start", "Normal", "Bypass"]);
    }

    #[test] fn cf_jtbd_02_invoice_exception_routing() {
        run_counterfactual_validation("JTBD-02 Order Recovery", 
            vec!["Normal", "Start"]);
    }

    #[test] fn cf_jtbd_03_compliance_gate_violation() {
        run_counterfactual_validation("JTBD-03 Mutual Exclusion Recovery", 
            vec!["Start", "Bypass", "Normal"]);
    }

    #[test] fn cf_jtbd_04_incident_mode_activation() {
        run_counterfactual_validation("JTBD-04 Repetition Recovery", 
            vec!["Start", "Bypass", "Bypass"]);
    }

    #[test] fn cf_jtbd_05_spaghetti_process_recovery() {
        run_counterfactual_validation("JTBD-05 Spaghetti Order Recovery", 
            vec!["End", "Start"]); // Illegal Petri start
    }

    #[test] fn cf_jtbd_06_streaming_capacity_budget() {
        run_counterfactual_validation("JTBD-06 Capacity Reset", 
            vec!["Start", "End", "Start"]); 
    }

    #[test] fn cf_jtbd_07_counterfactual_reroute_before_action() {
        run_counterfactual_validation("JTBD-07 Reroute Recovery", 
            vec!["Start", "Bypass", "Bypass"]);
    }

    #[test] fn cf_jtbd_08_human_handoff_under_ambiguity() {
        run_counterfactual_validation("JTBD-08 Ambiguity Recovery", 
            vec!["Normal", "Normal"]); 
    }

    #[test] fn cf_jtbd_09_object_centric_incident_timeline() {
        run_counterfactual_validation("JTBD-09 OCPM Semantic Recovery", 
            vec!["Start", "Normal", "Normal"]);
    }

    #[test] fn cf_jtbd_10_adversarial_noise_stream() {
        run_counterfactual_validation("JTBD-10 Noise Cleanup", 
            vec!["Start", "End", "End"]); 
    }

    #[test] fn cf_jtbd_11_batch_boundary_collapse() {
        run_counterfactual_validation("JTBD-11 Boundary Recovery", 
            vec!["Start", "End", "End"]); 
    }

    #[test] fn cf_jtbd_12_digital_team_role_conflict() {
        run_counterfactual_validation("JTBD-12 Conflict Resolution", 
            vec!["Start", "Normal", "Bypass"]); 
    }

    #[test] fn cf_jtbd_13_fully_autonomic_closed_loop() {
        run_counterfactual_validation("JTBD-13 Closed Loop Stabilization", 
            vec!["End", "Normal"]); 
    }

    #[test] fn cf_jtbd_14_forced_human_governance() {
        run_counterfactual_validation("JTBD-14 Governance Recovery", 
            vec!["Start", "Bypass", "Bypass"]);
    }

    #[test] fn cf_jtbd_15_petabyte_stream_approximation_mode() {
        run_counterfactual_validation("JTBD-15 Stream Stability Recovery", 
            vec!["Start", "Bypass", "Normal"]);
    }

    #[test] fn cf_jtbd_16_regression_reproduction() {
        run_counterfactual_validation("JTBD-16 Regression Recovery", 
            vec!["Normal", "Start"]);
    }
}
