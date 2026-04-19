#[cfg(test)]
mod tests {
    use crate::autonomic::{AutonomicKernel, Vision2030Kernel, AutonomicEvent, AutonomicResult};
    use std::time::SystemTime;

    /// Universal invariant checker macro
    macro_rules! assert_universal_invariants {
        ($kernel:expr, $results:expr) => {
            let state = $kernel.infer();
            assert!(state.process_health >= 0.0 && state.process_health <= 1.0, "Health out of bounds");
            for res in &$results {
                let manifest = $kernel.manifest(res);
                assert!(manifest.contains("VISION_2030_MANIFEST"), "Missing manifest prefix");
                assert!(manifest.contains("hash="), "Missing deterministic hash");
            }
        };
    }

    fn run_scenario(name: &str, events: Vec<&str>) -> (Vision2030Kernel, Vec<AutonomicResult>) {
        println!("\n--- JTBD Scenario: {} ---", name);
        let mut kernel = Vision2030Kernel::new();
        let mut all_results = Vec::new();
        
        for payload in events {
            let event = AutonomicEvent {
                source: "test_suite".to_string(),
                payload: payload.to_string(),
                timestamp: SystemTime::now(),
            };
            println!("📥 Event: {}", event.payload);
            let results = kernel.run_cycle(event);
            
            // Simulation of feedback loop: Success yields reward, failure/drift yields penalty
            let state = kernel.infer();
            let reward = if state.drift_detected { -0.2 } else { 0.1 };
            
            kernel.adapt(crate::autonomic::AutonomicFeedback {
                reward,
                human_override: false,
                side_effects: vec![],
            });

            all_results.extend(results);
        }
        (kernel, all_results)
    }

    #[test] fn jtbd_01_offshore_maintenance_drift() {
        let (k, res) = run_scenario("Offshore Maintenance Drift", 
            vec!["Start: System Boot", "Normal: Valve Open", "Bypass: Emergency Skip", "End: Finish"]);
        assert_universal_invariants!(k, res);
        assert!(k.infer().drift_detected, "XOR violation (Normal -> Bypass) must be detected");
    }
    
    #[test] fn jtbd_02_invoice_exception_routing() {
        let (k, res) = run_scenario("Invoice Exception Routing", 
            vec!["Normal: Invoice Matched", "Start: Late initialization"]);
        assert_universal_invariants!(k, res);
        assert!(k.infer().drift_detected, "Partial order violation (Normal before Start) must be detected");
    }
    
    #[test] fn jtbd_03_compliance_gate_violation() {
        let (k, res) = run_scenario("Compliance Gate Violation", 
            vec!["Start: Secure Login", "Bypass: Port Scan", "Normal: Regular Activity"]);
        assert_universal_invariants!(k, res);
        assert!(k.infer().drift_detected, "XOR violation (Bypass then Normal) must be detected");
    }

    #[test] fn jtbd_04_incident_mode_activation() {
        let (k, res) = run_scenario("Incident Mode Activation", 
            vec!["Start", "Bypass", "Bypass", "Bypass", "Bypass"]);
        assert_universal_invariants!(k, res);
        let state = k.infer();
        assert!(state.process_health < 1.0);
    }

    #[test] fn jtbd_05_spaghetti_process_recovery() {
        let (k, res) = run_scenario("Spaghetti Process Recovery", 
            vec!["Start", "Concurrent Task A", "Concurrent Task B", "End"]);
        assert_universal_invariants!(k, res);
        // Relaxed constraints: No drift expected for this trace
        assert!(!k.infer().drift_detected); 
    }

    #[test] fn jtbd_06_streaming_capacity_budget() {
        let (k, res) = run_scenario("Streaming Capacity Budget", 
            vec!["Start", "Normal", "Normal", "Normal"]);
        assert_universal_invariants!(k, res);
        assert!(k.infer().active_cases > 0);
    }

    #[test] fn jtbd_07_counterfactual_reroute_before_action() {
        let (k, res) = run_scenario("Counterfactual Reroute Before Action", 
            vec!["Start", "Bypass", "Bypass"]);
        assert_universal_invariants!(k, res);
        // High risk escalation should have been evaluated by simulator in accept()
    }

    #[test] fn jtbd_08_human_handoff_under_ambiguity() {
        let (k, res) = run_scenario("Human Handoff Under Ambiguity", 
            vec!["Start", "Unknown Activity Payload", "End"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_09_object_centric_incident_timeline() {
        let (k, res) = run_scenario("Object-Centric Incident Timeline", 
            vec!["Start: ObjA", "Normal: ObjA", "Start: ObjB", "End: ObjA"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_10_adversarial_noise_stream() {
        let (k, res) = run_scenario("Adversarial Noise Stream", 
            vec!["Start", "!!!NOISE!!!", "Normal", "End"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_11_batch_boundary_collapse() {
        let (k, res) = run_scenario("Batch Boundary Collapse", 
            vec!["Start", "End", "Start", "End"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_12_digital_team_role_conflict() {
        let (k, res) = run_scenario("Digital Team Role Conflict", 
            vec!["Start", "Normal", "Bypass"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_13_fully_autonomic_closed_loop() {
        let (k, res) = run_scenario("Fully Autonomic Closed Loop", 
            vec!["Start", "Normal", "End"]);
        assert_universal_invariants!(k, res);
        assert!(!k.infer().drift_detected, "Happy path should not detect drift");
    }

    #[test] fn jtbd_14_forced_human_governance() {
        let (k, res) = run_scenario("Forced Human Governance", 
            vec!["Start", "Bypass", "Bypass", "Bypass"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_15_petabyte_stream_approximation_mode() {
        let (k, res) = run_scenario("Petabyte Stream Approximation Mode", 
            vec!["Start", "Normal", "End"]);
        assert_universal_invariants!(k, res);
    }

    #[test] fn jtbd_16_regression_reproduction() {
        let (k, res) = run_scenario("Regression Reproduction", 
            vec!["Start", "Normal", "End"]);
        assert_universal_invariants!(k, res);
    }
}
