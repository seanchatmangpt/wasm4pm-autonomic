#[cfg(test)]
mod tests {
    use crate::autonomic::{AutonomicEvent, AutonomicKernel, Vision2030Kernel};
    use crate::utils::dense_kernel::fnv1a_64;

    /// Universal invariant checker macro
    macro_rules! assert_universal_invariants {
        ($kernel:expr, $_executed_count:expr) => {
            let state = $kernel.infer();
            assert!(
                state.process_health >= 0.0 && state.process_health <= 1.0,
                "Health out of bounds"
            );
        };
    }

    fn run_scenario(name: &str, events: Vec<&str>) -> (Vision2030Kernel<1>, u32) {
        println!("\n--- Combinatorial JTBD Scenario: {} ---", name);
        let mut kernel = Vision2030Kernel::<1>::new();
        let mut total_executed = 0;

        for (i, payload) in events.iter().enumerate() {
            let p = payload.to_lowercase();
            let activity_idx = if p.contains("start") {
                0
            } else if p.contains("normal") || p.contains("matched") {
                1
            } else if p.contains("bypass") || p.contains("skip") || p.contains("violation") {
                2
            } else if p.contains("end") || p.contains("limit") || p.contains("finish") {
                3
            } else if p.contains("task a") || p.contains("concurrenta") {
                4
            } else if p.contains("task b") || p.contains("concurrentb") {
                5
            } else {
                99 // Unknown
            };

            let payload_hash = fnv1a_64(payload.as_bytes());
            let event = AutonomicEvent {
                source_hash: (i % 3) as u64,
                activity_idx,
                payload_hash,
                timestamp_ns: 123456789,
            };
            println!("📥 Event [{}]: {} (idx={})", i, payload, activity_idx);
            let count = kernel.run_cycle(&event);
            total_executed += count;

            let state = kernel.infer();
            let reward = (state.conformance_score * 0.4) + (state.process_health * 0.4);

            kernel.adapt(&crate::autonomic::AutonomicFeedback {
                reward,
                human_override: false,
            });
        }
        (kernel, total_executed)
    }

    #[test]
    fn jtbd_01_offshore_maintenance_drift() {
        let events = vec![
            "Start: System Boot order creates",
            "Normal: Valve Open item updates value changed",
            "Bypass: Emergency Skip item reads critical",
            "End: Finish order",
        ];

        let (k, count) = run_scenario("Offshore Maintenance Drift Combinatorial", events);
        assert_universal_invariants!(k, count);

        let final_state = k.infer();
        assert!(
            final_state.drift_occurred,
            "Anomalies must be detected"
        );
    }

    #[test]
    fn jtbd_02_invoice_exception_routing() {
        let events = vec![
            "Normal: Invoice Matched item divergence",
            "Start: Late initialization order creates",
            "ConcurrentA: parallel routing item updates",
        ];

        let (k, count) = run_scenario("Invoice Exception Routing Combinatorial", events);
        assert_universal_invariants!(k, count);

        let final_state = k.infer();
        assert!(
            final_state.drift_occurred,
            "Divergence must compound to trigger drift"
        );
    }

    #[test]
    fn jtbd_03_compliance_gate_violation() {
        let events = vec![
            "Start: Secure Login order creates",
            "Bypass: Port Scan item relates to order",
            "Normal: Regular Activity item updates",
        ];

        let (k, count) = run_scenario("Compliance Gate Violation Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            final_state.drift_occurred,
            "XOR violation must trigger semantic drift"
        );
    }

    #[test]
    fn jtbd_04_incident_mode_activation() {
        let events = vec![
            "Start order item creates",
            "Bypass order updates",
            "Bypass item updates",
            "Bypass order item reads",
            "Bypass item critical",
        ];

        let (k, count) = run_scenario("Incident Mode Activation Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            final_state.process_health < 1.0,
            "Repeated bypasses must degrade health"
        );
    }

    #[test]
    fn jtbd_05_spaghetti_process_recovery() {
        let events = vec![
            "Start order creates",
            "Normal item matched",
            "Concurrent Task A item creates",
            "Concurrent Task B relates to order",
            "End order item reads",
        ];

        let (k, count) = run_scenario("Spaghetti Process Recovery Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            !final_state.drift_occurred,
            "Parallel interleaving with O2O relations is structurally sound"
        );
    }

    #[test]
    fn jtbd_06_streaming_capacity_budget() {
        let events = vec![
            "Start order creates",
            "Normal item updates",
            "Normal item updates",
            "Normal item updates",
        ];

        let (k, count) = run_scenario("Streaming Capacity Budget Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            final_state.throughput > 0.0,
            "Throughput tracked even during drift"
        );
        assert!(final_state.drift_occurred, "Repetition must be flagged");
    }

    #[test]
    fn jtbd_07_counterfactual_reroute_before_action() {
        let events = vec![
            "Normal item divergence",
            "Bypass order creates",
            "Bypass item updates critical",
        ];

        let (k, count) = run_scenario("Counterfactual Reroute Before Action Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            final_state.drift_occurred,
            "Combinatorial anomalies must trigger drift context"
        );
    }

    #[test]
    fn jtbd_08_human_handoff_under_ambiguity() {
        let events = vec![
            "Start order creates",
            "Unknown Activity Payload item relates to order",
            "End item reads",
        ];

        let (k, count) = run_scenario("Human Handoff Under Ambiguity Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            final_state.throughput > 0.0,
            "Throughput tracking survives"
        );
    }

    #[test]
    fn jtbd_09_object_centric_incident_timeline() {
        let events = vec![
            "Start order creates",
            "ConcurrentA item updates",
            "ConcurrentB order reads",
            "Normal item relates to order",
            "End order item reads",
        ];

        let (k, count) = run_scenario("Object-Centric Incident Timeline Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            !final_state.drift_occurred,
            "Valid OCEL 2.0 interleaving must not cause drift"
        );
    }

    #[test]
    fn jtbd_10_adversarial_noise_stream() {
        let events = vec![
            "Start order creates",
            "!!!NOISE!!! divergence critical",
            "Normal item updates",
            "End item reads",
        ];

        let (k, count) = run_scenario("Adversarial Noise Stream Combinatorial", events);
        assert_universal_invariants!(k, count);
        assert!(count > 0);
    }

    #[test]
    fn jtbd_11_batch_boundary_collapse() {
        let events = vec![
            "Start order creates",
            "End item reads",
            "Start order updates",
            "End item critical",
        ];

        let (k, count) = run_scenario("Batch Boundary Collapse Combinatorial", events);
        assert_universal_invariants!(k, count);
        assert!(count > 0);
    }

    #[test]
    fn jtbd_12_digital_team_role_conflict() {
        let events = vec![
            "Start order creates",
            "Normal order updates",
            "Bypass item updates",
        ];

        let (k, count) = run_scenario("Digital Team Role Conflict Combinatorial", events);
        assert_universal_invariants!(k, count);
        assert!(count > 0);
    }

    #[test]
    fn jtbd_13_fully_autonomic_closed_loop() {
        let events = vec![
            "Start order creates",
            "Normal item relates to order",
            "End item reads",
        ];

        let (k, count) = run_scenario("Fully Autonomic Closed Loop Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            !final_state.drift_occurred,
            "Complex valid combinatorial path should run autonomously"
        );
        assert!(final_state.process_health > 0.0, "Health is preserved");
    }

    #[test]
    fn jtbd_14_forced_human_governance() {
        let events = vec![
            "Start order creates",
            "Bypass item updates",
            "Bypass item value changed critical",
            "Bypass order divergence critical",
        ];

        let (k, count) = run_scenario("Forced Human Governance Combinatorial", events);
        assert_universal_invariants!(k, count);
        assert!(count > 0);
    }

    #[test]
    fn jtbd_15_petabyte_stream_approximation_mode() {
        let events = vec![
            "Start order creates",
            "Normal item matched",
            "ConcurrentA item updates",
            "ConcurrentB order relates to item",
            "End order item reads",
        ];

        let (k, count) = run_scenario("Petabyte Stream Approximation Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            !final_state.drift_occurred,
            "Approximation remains stable under O2O load"
        );
    }

    #[test]
    fn jtbd_16_regression_reproduction() {
        let events = vec![
            "Start item creates",
            "Normal item updates",
            "End item reads",
        ];

        let (k, count) = run_scenario("Regression Reproduction Combinatorial", events);
        assert_universal_invariants!(k, count);
        let final_state = k.infer();
        assert!(
            !final_state.drift_occurred,
            "Baseline reproduction must pass combinatorial checks"
        );
    }

    #[test]
    fn jtbd_17_object_centric_divergence() {
        let events = vec![
            "Start order creates",
            "Normal item divergence",
            "End order reads",
        ];

        let (k, _count) = run_scenario("Object-Centric Divergence Combinatorial", events);
        assert_universal_invariants!(k, _count);
    }

    #[test]
    fn jtbd_18_ocel20_predictive_prescriptive() {
        let events = vec![
            "Start order creates",
            "Normal item relates to order",
            "Normal amount value changed critical",
            "End item reads",
        ];

        let (k, count) = run_scenario("OCEL 2.0 Predictive Prescriptive Combinatorial", events);
        assert_universal_invariants!(k, count);
    }
}
