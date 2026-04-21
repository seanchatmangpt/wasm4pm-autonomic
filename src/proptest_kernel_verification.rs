#[cfg(test)]
mod proptests {
<<<<<<< HEAD
<<<<<<< HEAD
    use crate::dteam::kernel::branchless::apply_branchless_update;
    use crate::models::petri_net::FlatIncidenceMatrix;
=======
    use crate::dteam::kernel::branchless::{apply_branchless_update, apply_ktier_update};
    use crate::models::petri_net::{FlatIncidenceMatrix, PetriNet, Place, Transition, Arc};
>>>>>>> wreckit/branchless-state-equation-calculus-eliminate-conditional-logic-in-petrinet-verification
    use crate::reinforcement::WorkflowAction;
    use crate::utils::dense_kernel::KBitSet;
    use crate::{RlAction, RlState};
    use proptest::prelude::*;

=======
    use crate::reinforcement::{WorkflowAction, WorkflowState};
    use crate::{RlAction, RlState};
    use crate::dteam::core::KTier;
    use crate::dteam::kernel::branchless::{fire_transition, transition_rl_state};
    use crate::models::petri_net::{PetriNet, Transition, Arc};
    use proptest::prelude::*;

    // μ-kernel invariant: Var(τ) == 0 (Deterministic Execution)
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
    proptest! {
        #[test]
        fn test_μ_kernel_determinism_k64(
            h in 0i8..5,
            a in 0usize..3,
        ) {
<<<<<<< HEAD
            let state = RlState::<1> {
                health_level: h,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
<<<<<<< HEAD
                marking_mask: KBitSet::zero(),
=======
                marking_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
                activities_hash: 0,
                ontology_mask: KBitSet::zero(),
                universe: None,
            };
            let action = RlAction::from_index(a).unwrap();
<<<<<<< HEAD

            // Execute twice to check variancy τ
=======
            
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
            let result1 = transition(state, action);
            let result2 = transition(state, action);
=======
            let state = create_test_state::<1>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K64)");
        }

        #[test]
        fn test_μ_kernel_determinism_k128(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<2>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K128)");
        }
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond

        #[test]
        fn test_μ_kernel_determinism_k256(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<4>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K256)");
        }

        #[test]
        fn test_μ_kernel_determinism_k512(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<8>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K512)");
        }

        #[test]
        fn test_μ_kernel_determinism_k1024(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<16>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K1024)");
        }

        #[test]
        fn test_μ_kernel_determinism_k2048(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<32>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K2048)");
        }

        #[test]
        fn test_μ_kernel_determinism_k4096(
            h in 0i8..5,
            a in 0usize..3,
        ) {
            let state = create_test_state::<64>(h);
            let action = RlAction::from_index(a).unwrap();
            let result1 = state.step(action);
            let result2 = state.step(action);
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (K4096)");
        }
<<<<<<< HEAD
<<<<<<< HEAD
=======
=======

        #[test]
        fn test_branchless_fire_admissibility(
            m in 0u64..u64::MAX,
            i in 0u64..u64::MAX,
            o in 0u64..u64::MAX,
        ) {
            let mut state = RlState {
                marking_mask: m,
                activities_hash: 0,
                health_level: 0,
                event_rate_q: 0,
                activity_count_q: 0,
                spc_alert_level: 0,
                drift_status: 0,
                rework_ratio_q: 0,
                circuit_state: 0,
                cycle_phase: 0,
            };

            let is_enabled = (m & i) == i;
            let fired = crate::dteam::kernel::branchless::apply_branchless_fire(&mut state, i, o);
            
            assert_eq!(fired, is_enabled);
            if is_enabled {
                assert_eq!(state.marking_mask, (m & !i) | o);
            } else {
                assert_eq!(state.marking_mask, m);
            }
        }

        #[test]
        fn test_rl_action_admissibility(
            h in -10i8..10i8,
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
<<<<<<< HEAD
<<<<<<< HEAD
            
            let admissible = state.is_admissible(action);
            match action {
                RlAction::Idle => assert!(admissible),
                RlAction::Optimize => assert_eq!(admissible, h < 5),
                RlAction::Rework => assert_eq!(admissible, h > 0),
            }
=======
=======
>>>>>>> wreckit/mdl-refinement-upgrade-structural-scoring-in-src-models-petri-net-rs-to-follow-φ-n-exactly

            // Execute twice to check variancy τ
<<<<<<< HEAD
            let result1 = transition(state, action);
            let result2 = transition(state, action);

            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic");
>>>>>>> wreckit/cryptographic-execution-provenance-enhance-executionmanifest-with-full-h-l-π-h-n-hashing
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
=======
            let result1 = transition_rl_state(state, action);
            let result2 = transition_rl_state(state, action);
            
            assert_eq!(result1, result2, "Kernel μ failed: transition not deterministic (Var(τ) != 0)");
        }

        #[test]
        fn test_branchless_transition_firing(
            marking in 0u64..u64::MAX,
            input_mask in 0u64..u64::MAX,
            output_mask in 0u64..u64::MAX,
        ) {
            let result = fire_transition(marking, input_mask, output_mask);
            
            // Canonical check
            let enabled = (marking & input_mask) == input_mask;
            let expected = if enabled {
                (marking & !input_mask) | output_mask
            } else {
                marking
            };
            
            assert_eq!(result, expected, "Branchless firing logic divergent from canonical reference");
        }

        #[test]
        fn test_mdl_minimality_invariant(
            t_count in 1usize..100,
            a_count in 1usize..200,
        ) {
            let mut net = PetriNet::default();
            for i in 0..t_count {
                net.transitions.push(Transition {
                    id: format!("t{}", i),
                    label: "A".to_string(),
                    is_invisible: None,
                });
            }
            for i in 0..a_count {
                net.arcs.push(Arc {
                    from: "p1".to_string(),
                    to: format!("t{}", i % t_count),
                    weight: None,
                });
            }
            
            let score = net.mdl_score();
            let expected = (t_count as f64) + (a_count as f64 * (t_count as f64).log2());
            
            assert!((score - expected).abs() < f64::EPSILON, "MDL score invariant violation");
        }

        #[test]
        fn test_ktier_alignment_and_capacity(
            k_idx in 0..5usize,
        ) {
            let tiers = [KTier::K64, KTier::K128, KTier::K256, KTier::K512, KTier::K1024];
            let tier = tiers[k_idx];
            
            let expected_capacity = match tier {
                KTier::K64 => 64,
                KTier::K128 => 128,
                KTier::K256 => 256,
                KTier::K512 => 512,
                KTier::K1024 => 1024,
            };
            
            assert_eq!(tier.capacity(), expected_capacity, "KTier capacity misalignment");
            assert_eq!(tier.capacity() % 64, 0, "KTier word alignment violation");
>>>>>>> wreckit/deterministic-kernel-μ-verification-create-cross-architecture-test-suite-to-verify-var-τ-0
        }
    }
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings

<<<<<<< HEAD
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
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o

        #[test]
        fn test_branchless_kernel_equation_parity(
            mask in 0u64..1024,
            transitions in 1usize..8,
        ) {
            let places_count = 10;
            // Generate a random incidence matrix
            let mut data = vec![0i32; places_count * transitions];
            let mut input_masks = vec![KBitSet::<16>::zero(); transitions];
            let mut output_masks = vec![KBitSet::<16>::zero(); transitions];

            for t in 0..transitions {
                for p in 0..places_count {
                    let val = if (t + p) % 3 == 0 { -1 } else if (t + p) % 3 == 1 { 1 } else { 0 };
                    data[p * transitions + t] = val;
                    if val < 0 {
                        let _ = input_masks[t].set(p);
                    } else if val > 0 {
                        let _ = output_masks[t].set(p);
                    }
                }
            }

            let incidence = FlatIncidenceMatrix {
                data,
                places_count,
                transitions_count: transitions,
                input_masks,
                output_masks,
            };

            let transition_idx = 0; // Test first transition
            let result1 = apply_branchless_update(mask, transition_idx, &incidence);
            let result2 = apply_branchless_update(mask, transition_idx, &incidence);
            
            assert_eq!(result1, result2, "Branchless transition failed: not deterministic");
            
            // Verification parity: manually compute the next mask
            let mut expected = mask;
            let in_mask = incidence.input_masks[transition_idx].words[0];
            let out_mask = incidence.output_masks[transition_idx].words[0];
            expected = (expected & !in_mask) | out_mask;
            assert_eq!(result1, expected);
        }

        #[test]
        fn test_ktier_branchless_updates(
            w_idx in 0..16usize,
            bit_idx in 0..64usize,
        ) {
            let mut marking = KBitSet::<16>::zero();
            let _ = marking.set(w_idx * 64 + bit_idx);
            
            let mut input_masks = vec![KBitSet::<16>::zero(); 1];
            let mut output_masks = vec![KBitSet::<16>::zero(); 1];
            
            // Transition consumes the set bit and produces one in the next word (circular)
            let _ = input_masks[0].set(w_idx * 64 + bit_idx);
            let next_bit = (w_idx * 64 + bit_idx + 1) % 1024;
            let _ = output_masks[0].set(next_bit);
            
            let incidence = FlatIncidenceMatrix {
                data: vec![0; 1024],
                places_count: 1024,
                transitions_count: 1,
                input_masks,
                output_masks,
            };
            
            let next_marking = apply_ktier_update::<16>(marking, 0, &incidence);
            
            assert!(!next_marking.contains(w_idx * 64 + bit_idx), "Should have consumed token");
            assert!(next_marking.contains(next_bit), "Should have produced token");
            assert_eq!(next_marking.words.iter().map(|w| w.count_ones()).sum::<u32>(), 1);
        }

        #[test]
        fn test_structural_workflow_net_branchless_verification(
            p_count in 2usize..20,
            t_count in 1usize..10,
        ) {
            let mut net = PetriNet::default();
            for i in 0..p_count {
                net.places.push(Place { id: format!("p{}", i) });
            }
            for i in 0..t_count {
                net.transitions.push(Transition { id: format!("t{}", i), label: format!("T{}", i), is_invisible: None });
            }
            
            // Create a simple chain: p0 -> t0 -> p1 -> t1 -> ... -> pN
            for i in 0..t_count {
                net.arcs.push(Arc { from: format!("p{}", i), to: format!("t{}", i), weight: None });
                net.arcs.push(Arc { from: format!("t{}", i), to: format!("p{}", i+1), weight: None });
            }
            
            // This is a workflow net if p0 is source and pN is sink
            // Wait, we need to handle the case where p_count > t_count + 1
            
            net.compile_incidence();
            let is_wf = net.is_structural_workflow_net();
            
            if p_count == t_count + 1 {
                assert!(is_wf, "Simple chain should be a workflow net (p={}, t={})", p_count, t_count);
            }
            
            let calculus_ok = net.verifies_state_equation_calculus();
            if is_wf {
                assert!(calculus_ok);
            }
        }
    }
<<<<<<< HEAD
    fn transition(state: RlState<1>, action: RlAction) -> RlState<1> {
        let mut next = state;
        match action {
            RlAction::Idle => (),
            RlAction::Optimize => next.health_level += 1,
            RlAction::Rework => next.health_level = (next.health_level - 1).max(0),
=======
    #[test]
    fn test_zero_allocation_hot_path_verification() {
        let state = RlState {
            health_level: 1,
=======

    fn create_test_state<const W: usize>(h: i8) -> RlState<W> {
        RlState::<W> {
            health_level: h,
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
            event_rate_q: 0,
            activity_count_q: 0,
            spc_alert_level: 0,
            drift_status: 0,
            rework_ratio_q: 0,
            circuit_state: 0,
            cycle_phase: 0,
<<<<<<< HEAD
            marking_mask: 0,
            activities_hash: 0,
        };
        let action = RlAction::Optimize;
        let next = transition_rl_state(state, action);
        assert_eq!(next.health_level, 2);
    }

    #[test]
    fn test_provenance_manifest_emission() {
        use crate::dteam::orchestration::{Engine, EngineResult};
        use crate::models::{Event, EventLog, Trace};

        let engine = Engine::builder().with_k_tier(64).build();
        let mut log = EventLog::default();
        let mut trace = Trace::default();
        trace.events.push(Event::new("A".to_string()));
        log.add_trace(trace);

        let result = engine.run(&log);
        if let EngineResult::Success(_, manifest) = result {
            assert_eq!(manifest.input_log_hash, log.canonical_hash());
            assert!(!manifest.action_sequence.is_empty());
            assert!(manifest.mdl_score > 0.0);
            assert!(manifest.latency_ns > 0);
        } else {
            panic!("Engine execution failed");
>>>>>>> wreckit/deterministic-kernel-μ-verification-create-cross-architecture-test-suite-to-verify-var-τ-0
=======
            marking_mask: KBitSet::zero(),
            activities_hash: 0,
>>>>>>> wreckit/k-tier-scalability-optimize-bitset-alignment-for-k-1024-and-beyond
        }
    }
}
