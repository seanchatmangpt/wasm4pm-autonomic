#[cfg(test)]
mod tests {
    use crate::autonomic::{AutonomicEvent, AutonomicKernel, AutonomicResult, Vision2030Kernel};

    use std::time::SystemTime;

    /// Universal invariant checker macro
    macro_rules! assert_universal_invariants {
        ($kernel:expr, $results:expr) => {
            let state = $kernel.infer();
            assert!(
                state.process_health >= 0.0 && state.process_health <= 1.0,
                "Health out of bounds"
            );
            for res in &$results {
                let manifest = $kernel.manifest(res);
                assert!(
                    manifest.contains("VISION_2030_MANIFEST"),
                    "Missing manifest prefix"
                );
                assert!(manifest.contains("hash="), "Missing deterministic hash");
            }
        };
    }

    fn run_scenario(name: &str, events: Vec<&str>) -> (Vision2030Kernel<1>, Vec<AutonomicResult>) {
        println!("\n--- Combinatorial JTBD Scenario: {} ---", name);
        let mut kernel = Vision2030Kernel::<1>::new();
        let mut all_results = Vec::new();

        for (i, payload) in events.iter().enumerate() {
            let event = AutonomicEvent {
                source: format!("test_suite_agent_{}", i % 3),
                payload: payload.to_string(),
                timestamp: SystemTime::now(),
            };
            println!("📥 Event [{}]: {}", i, event.payload);
            let results = kernel.run_cycle(event);

            let state = kernel.infer();
            // Combinatorial reward function forcing the RL agent to balance multiple dimensions
            let mut reward = (state.conformance_score * 0.4) + (state.process_health * 0.4);
            if state.drift_detected {
                reward -= 0.5;
            }
            if state.active_cases > 10 {
                reward -= 0.1;
            } // Penalize extreme bottlenecks

            kernel.adapt(crate::autonomic::AutonomicFeedback {
                reward,
                human_override: false,
                side_effects: vec![],
            });

            all_results.extend(results);
        }
        (kernel, all_results)
    }

    #[test]
    fn jtbd_01_offshore_maintenance_drift() {
        // Combinatorial Scenario: POWL XOR Violation + OCEL Attribute Change
        // Tests interaction between control-flow drift and predictive prescriptive attributes.
        let events = vec![
            "Start: System Boot order creates",
            "Normal: Valve Open item updates value changed",
            "Bypass: Emergency Skip item reads critical", // Triggers POWL XOR and OCPM Critical Attribute
            "End: Finish order",
        ];

        let (k, res) = run_scenario("Offshore Maintenance Drift Combinatorial", events);
        assert_universal_invariants!(k, res);

        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "XOR violation combined with critical attribute must be detected"
        );
        assert!(
            final_state.process_health < 0.9,
            "Health must degrade from combinatorial anomalies"
        );
    }

    #[test]
    fn jtbd_02_invoice_exception_routing() {
        // Combinatorial Scenario: POWL Partial Order + OC-DFG Divergence
        let events = vec![
            "Normal: Invoice Matched item divergence", // Triggers Partial Order (No Start) + OCPM Divergence
            "Start: Late initialization order creates",
            "ConcurrentA: parallel routing item updates",
        ];

        let (k, res) = run_scenario("Invoice Exception Routing Combinatorial", events);
        assert_universal_invariants!(k, res);

        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "Partial order and divergence must compound to trigger drift"
        );
        assert!(
            final_state.conformance_score < 1.0,
            "SWAR Token replay must reject invalid partial order"
        );
    }

    #[test]
    fn jtbd_03_compliance_gate_violation() {
        // Combinatorial Scenario: O2O Structural Relations + Compliance XOR
        let events = vec![
            "Start: Secure Login order creates",
            "Bypass: Port Scan item relates to order", // Triggers O2O relation and Bypass XOR path
            "Normal: Regular Activity item updates",   // Violates XOR
        ];

        let (k, res) = run_scenario("Compliance Gate Violation Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "O2O interwoven with XOR violation must trigger semantic drift"
        );
    }

    #[test]
    fn jtbd_04_incident_mode_activation() {
        // Combinatorial Scenario: Repetition Exclusion + Multi-Object Saturation
        let events = vec![
            "Start order item creates",
            "Bypass order updates",
            "Bypass item updates", // POWL Repetition exclusion triggered
            "Bypass order item reads",
            "Bypass item critical",
        ];

        let (k, res) = run_scenario("Incident Mode Activation Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.process_health < 0.8,
            "Repeated bypasses across multiple objects must severely degrade health"
        );
    }

    #[test]
    fn jtbd_05_spaghetti_process_recovery() {
        // Combinatorial Scenario: Parallel Interleaving + O2O structural binding
        let events = vec![
            "Start order creates",
            "Concurrent Task A item creates",
            "Concurrent Task B relates to order", // O2O structural binding during parallel execution
            "End order item reads",
        ];

        let (k, res) = run_scenario("Spaghetti Process Recovery Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !final_state.drift_detected,
            "Parallel interleaving with O2O relations is structurally sound"
        );
    }

    #[test]
    fn jtbd_06_streaming_capacity_budget() {
        // Combinatorial Scenario: CMS Estimation + Repetition Violation
        let events = vec![
            "Start order creates",
            "Normal item updates",
            "Normal item updates", // Triggers POWL repetition drift but maintains throughput
            "Normal item updates",
        ];

        let (k, res) = run_scenario("Streaming Capacity Budget Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.active_cases > 0,
            "CMS must track active cases even during semantic drift"
        );
        assert!(final_state.drift_detected, "Repetition must be flagged");
    }

    #[test]
    fn jtbd_07_counterfactual_reroute_before_action() {
        // Combinatorial Scenario: High-Risk Context from OCPM divergence + POWL order
        let events = vec![
            "Normal item divergence",       // Triggers OCPM divergence
            "Bypass order creates",         // Triggers XOR and Order
            "Bypass item updates critical", // Triggers prescriptive attribute drift
        ];

        let (k, res) = run_scenario("Counterfactual Reroute Before Action Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "Combinatorial anomalies must trigger drift context"
        );
        assert!(
            final_state.conformance_score < 0.8,
            "Conformance score must plummet from SWAR invalid transitions"
        );
    }

    #[test]
    fn jtbd_08_human_handoff_under_ambiguity() {
        // Combinatorial Scenario: Ambiguous payload combined with O2O struct injection
        let events = vec![
            "Start order creates",
            "Unknown Activity Payload item relates to order",
            "End item reads",
        ];

        let (k, res) = run_scenario("Human Handoff Under Ambiguity Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.throughput > 0.0,
            "Throughput tracking survives ambiguous O2O injections"
        );
    }

    #[test]
    fn jtbd_09_object_centric_incident_timeline() {
        // Combinatorial Scenario: Complex Multi-Object Interleaving without violations
        let events = vec![
            "Start order creates",
            "ConcurrentA item updates",
            "ConcurrentB order reads",
            "Normal item relates to order",
            "End order item reads",
        ];

        let (k, res) = run_scenario("Object-Centric Incident Timeline Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !final_state.drift_detected,
            "Valid OCEL 2.0 interleaving must not cause drift"
        );
        assert!(
            final_state.active_cases > 0,
            "CMS scales across multiple object bindings"
        );
    }

    #[test]
    fn jtbd_10_adversarial_noise_stream() {
        // Combinatorial Scenario: Pure Noise vs Attribute Mutations
        let events = vec![
            "Start order creates",
            "!!!NOISE!!! divergence critical", // Injects fake critical payload
            "Normal item updates",
            "End item reads",
        ];

        let (k, res) = run_scenario("Adversarial Noise Stream Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        // The noise triggers a critical drift, which the agent immediately repairs.
        // We assert that the engine survived, maintained throughput, and executed a repair.
        assert!(
            !res.is_empty(),
            "Adversarial critical noise must trigger drift protection and a subsequent repair"
        );
        assert!(
            final_state.throughput > 0.0,
            "Throughput tracking survives noise injection"
        );
    }

    #[test]
    fn jtbd_11_batch_boundary_collapse() {
        // Combinatorial Scenario: Rapid Repetition + Structural Violation
        let events = vec![
            "Start order creates",
            "End item reads",
            "Start order updates", // Boundary collapse triggers POWL rule
            "End item critical",
        ];

        let (k, res) = run_scenario("Batch Boundary Collapse Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !res.is_empty(),
            "Boundary collapse detected via semantic constraints"
        );
        assert!(
            final_state.process_health < 1.0,
            "Health impacted by collapsed boundary"
        );
    }

    #[test]
    fn jtbd_12_digital_team_role_conflict() {
        // Combinatorial Scenario: Role conflict over multiple objects
        let events = vec![
            "Start order creates",
            "Normal order updates", // Team 1 takes normal path
            "Bypass item updates",  // Team 2 forces bypass on related object
        ];

        let (k, res) = run_scenario("Digital Team Role Conflict Combinatorial", events);
        assert_universal_invariants!(k, res);
        let _final_state = k.infer();
        assert!(
            !res.is_empty(),
            "XOR conflict across objects is detected and repaired"
        );
    }

    #[test]
    fn jtbd_13_fully_autonomic_closed_loop() {
        // Combinatorial Scenario: The ultimate valid complex trace
        let events = vec![
            "Start order creates",
            "Normal item relates to order",
            "End item reads",
        ];

        let (k, res) = run_scenario("Fully Autonomic Closed Loop Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !final_state.drift_detected,
            "Complex valid combinatorial path should run autonomously"
        );
        assert!(final_state.process_health > 0.0, "Health is preserved");
    }

    #[test]
    fn jtbd_14_forced_human_governance() {
        // Combinatorial Scenario: Critical attribute escalation forcing handoff
        let events = vec![
            "Start order creates",
            "Bypass item updates",
            "Bypass item value changed critical",
            "Bypass order divergence critical",
        ];

        let (k, res) = run_scenario("Forced Human Governance Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !res.is_empty(),
            "Critical divergence forces governance drift and repair"
        );
        // Due to repeated critical penalties combined with intermittent repairs, health stabilizes lower.
        assert!(
            final_state.process_health <= 0.65,
            "Health significantly penalised by combinatorial failure"
        );
    }

    #[test]
    fn jtbd_15_petabyte_stream_approximation_mode() {
        // Combinatorial Scenario: CMS under valid but heavy stream
        let events = vec![
            "Start order creates",
            "ConcurrentA item updates",
            "ConcurrentB order relates to item",
            "End order item reads",
        ];

        let (k, res) = run_scenario("Petabyte Stream Approximation Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !final_state.drift_detected,
            "Approximation remains stable under O2O load"
        );
        assert!(
            final_state.throughput > 3.0,
            "Throughput tracks massive scale"
        );
    }

    #[test]
    fn jtbd_16_regression_reproduction() {
        // Combinatorial Scenario: Baseline validation against all features
        let events = vec![
            "Start item creates",
            "Normal item updates",
            "End item reads",
        ];

        let (k, res) = run_scenario("Regression Reproduction Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            !final_state.drift_detected,
            "Baseline reproduction must pass combinatorial checks"
        );
    }

    #[test]
    fn jtbd_17_object_centric_divergence() {
        // Combinatorial Scenario: N:M Divergence explicit threshold breaking
        let events = vec![
            "Start order creates",
            "Normal item divergence", // Will spawn >5 bindings artificially
            "End order reads",
        ];

        let (k, res) = run_scenario("Object-Centric Divergence Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "Explicit OCPM divergence anomaly threshold breached"
        );
    }

    #[test]
    fn jtbd_18_ocel20_predictive_prescriptive() {
        // Combinatorial Scenario: Full OCEL 2.0 lifecycle with value change anomaly
        let events = vec![
            "Start order creates",
            "Normal item relates to order",
            "Normal amount value changed critical", // Attribute change forces predictive constraint
            "End item reads",
        ];

        let (k, res) = run_scenario("OCEL 2.0 Predictive Prescriptive Combinatorial", events);
        assert_universal_invariants!(k, res);
        let final_state = k.infer();
        assert!(
            final_state.drift_detected,
            "OCEL 2.0 attribute change drift precisely caught"
        );
        assert!(
            final_state.throughput > 0.0,
            "Throughput maintained during prescriptive tracking"
        );
    }

    #[test]
    fn jtbd_19_evidence_recovery() {
        use crate::autonomic::AutonomicAction;
        use crate::io::prediction_log::blake3_input_hash;

        println!("\n--- JTBD-19: Evidence Recovery (Retriever Cognitive Breed) ---");

        let mut kernel = Vision2030Kernel::<1>::new();

        // Pre-seed prediction log with 4 positive entries encoding high conformance
        let current_time_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        // Encode conformance as provenance_hash: (conformance as f32 / 1.0) * u32::MAX as f32
        let high_conformance_hash = (0.95f32 * u32::MAX as f32) as u64;

        for i in 0..4 {
            let input_bytes = format!("recovery_entry_{}", i).into_bytes();
            let input_hash_bytes = blake3_input_hash(&input_bytes);

            kernel.prediction_log.log_prediction(
                input_hash_bytes,
                current_time_us + i as u64 * 1000,
                true, // decision=true (positive entries)
                0,    // tier_fired=0
                high_conformance_hash,
            );
        }

        // Run clean sequence to establish healthy state
        let clean_events = vec![
            "Start: System Boot order creates",
            "Normal: Valve Open item updates",
            "End: Finish order",
        ];

        for (i, payload) in clean_events.iter().enumerate() {
            let event = AutonomicEvent {
                source: format!("recovery_agent_{}", i),
                payload: payload.to_string(),
                timestamp: SystemTime::now(),
            };
            kernel.observe(event);
        }

        let healthy_state = kernel.infer();
        println!(
            "  Healthy State: health={:.2}, conformance={:.2}, drift={}",
            healthy_state.process_health, healthy_state.conformance_score, healthy_state.drift_detected
        );

        // Inject drift by triggering Bypass violations 3 times
        for _ in 0..3 {
            kernel.observe(AutonomicEvent {
                source: "drift_injector".to_string(),
                payload: "Bypass: Emergency Skip critical".to_string(),
                timestamp: SystemTime::now(),
            });
        }

        let drifted_state = kernel.infer();
        assert!(
            drifted_state.drift_detected,
            "Drift must be detected after 3 Bypass violations"
        );
        let pre_recover_score = drifted_state.conformance_score;
        println!(
            "  Drifted State: health={:.2}, conformance={:.2}, drift={}",
            drifted_state.process_health, drifted_state.conformance_score, drifted_state.drift_detected
        );

        // Execute Recover action
        let recover_action = AutonomicAction::recover(200, "Evidence recovery from audit log");
        let result = kernel.execute(recover_action);

        // Verify recovery
        let recovered_state = kernel.infer();
        println!(
            "  Recovered State: health={:.2}, conformance={:.2}, drift={}",
            recovered_state.process_health, recovered_state.conformance_score, recovered_state.drift_detected
        );

        assert!(
            recovered_state.conformance_score > pre_recover_score,
            "Conformance score must improve after recovery"
        );
        assert!(
            !recovered_state.drift_detected,
            "Drift must be cleared after recovery action"
        );

        let manifest = kernel.manifest(&result);
        assert!(
            manifest.contains("VISION_2030_MANIFEST"),
            "Recovery must produce a valid manifest"
        );
    }
}
