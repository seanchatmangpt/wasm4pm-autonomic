//! End-to-end pipeline test: OCEL log → corpus → motifs → candidate
//! policy → gauntlet admission → field-pack compile → registry.
//!
//! Anti-stub: every step asserts byte-level or response-class consequences.
//! A stub that returns `Ok(())` at any stage fails downstream.

use autoinstinct::compile::{compile, CompileInputs};
use autoinstinct::corpus::{Episode, TraceCorpus};
use autoinstinct::drift::{DriftMonitor, Outcome};
use autoinstinct::gauntlet;
use autoinstinct::jtbd::JtbdScenario;
use autoinstinct::motifs::discover;
use autoinstinct::ocel::{validate, OcelEvent, OcelLog, OcelObject};
use autoinstinct::registry::PackRegistry;
use autoinstinct::synth::synthesize;
use autoinstinct::AutonomicInstinct;

#[test]
fn e2e_supply_chain_pack_compiles_and_registers() {
    // 1. OCEL world — supply-chain dock obstruction scenario.
    let log = OcelLog {
        objects: vec![
            OcelObject {
                iri: "https://schema.org/Vehicle/truck-1".into(),
                object_type: "https://schema.org/Vehicle".into(),
            },
            OcelObject {
                iri: "https://schema.org/Place/dock-A".into(),
                object_type: "https://schema.org/Place".into(),
            },
        ],
        events: vec![
            OcelEvent {
                iri: "urn:blake3:e-camera-detect".into(),
                event_type: "https://schema.org/Action".into(),
                objects: vec![
                    "https://schema.org/Vehicle/truck-1".into(),
                    "https://schema.org/Place/dock-A".into(),
                ],
                timestamp: "2026-04-29T12:00:00Z".into(),
            },
            OcelEvent {
                iri: "urn:blake3:e-drone-confirm".into(),
                event_type: "https://schema.org/Action".into(),
                objects: vec!["https://schema.org/Place/dock-A".into()],
                timestamp: "2026-04-29T12:01:00Z".into(),
            },
        ],
    };
    validate(&log).expect("OCEL log must validate");

    // 2. Trace corpus — hand-built from the OCEL log into closed-context
    // fingerprints. (Phase 2 will derive these mechanically; for now we
    // assert the handshake.)
    let mut corpus = TraceCorpus::new();
    for _ in 0..5 {
        corpus.push(Episode {
            context_urn: "urn:blake3:dock-obstruction-confirmed".into(),
            response: AutonomicInstinct::Inspect,
            receipt_urn: "urn:blake3:r-1".into(),
            outcome: Some("earned".into()),
        });
    }
    for _ in 0..3 {
        corpus.push(Episode {
            context_urn: "urn:blake3:dock-clear".into(),
            response: AutonomicInstinct::Ignore,
            receipt_urn: "urn:blake3:r-2".into(),
            outcome: Some("earned".into()),
        });
    }

    // 3. Motif discovery.
    let motifs = discover(&corpus, 3);
    assert_eq!(motifs.motifs.len(), 2, "two motifs above support 3");
    assert_eq!(motifs.motifs[0].support, 5);
    assert_eq!(motifs.motifs[0].response, AutonomicInstinct::Inspect);

    // 4. Candidate policy synthesis.
    let policy = synthesize(&motifs);
    assert_eq!(
        policy.select("urn:blake3:dock-obstruction-confirmed"),
        AutonomicInstinct::Inspect
    );
    assert_eq!(
        policy.select("urn:blake3:dock-clear"),
        AutonomicInstinct::Ignore
    );
    assert_eq!(
        policy.select("urn:blake3:never-seen"),
        AutonomicInstinct::Ignore,
        "default fallback"
    );

    // 5. Generated JTBD scenarios.
    let scenarios = vec![
        JtbdScenario {
            name: "dock-obstruction-inspects".into(),
            context_urn: "urn:blake3:dock-obstruction-confirmed".into(),
            expected: AutonomicInstinct::Inspect,
            perturbed_context_urn: "urn:blake3:dock-clear".into(),
            forbidden: vec![AutonomicInstinct::Refuse, AutonomicInstinct::Escalate],
        },
        JtbdScenario {
            name: "dock-clear-ignores".into(),
            context_urn: "urn:blake3:dock-clear".into(),
            expected: AutonomicInstinct::Ignore,
            perturbed_context_urn: "urn:blake3:dock-obstruction-confirmed".into(),
            forbidden: vec![AutonomicInstinct::Refuse],
        },
    ];

    // 6. Gauntlet admission.
    let report = gauntlet::run(&policy, &scenarios);
    assert!(
        report.admitted(),
        "supply-chain policy must pass gauntlet; counterexamples: {:?}",
        report.counterexamples
    );

    // 7. Compile to field-pack artifact.
    let pack = compile(CompileInputs {
        name: "supply-chain",
        ontology_profile: &[
            "http://www.w3.org/ns/prov#",
            "https://schema.org/",
            "urn:blake3:",
        ],
        admitted_breeds: &["mycin", "strips"],
        policy: &policy,
    });
    assert!(pack.digest_urn.starts_with("urn:blake3:"));
    assert_eq!(pack.autoinstinct_version, autoinstinct::AUTOINSTINCT_VERSION);

    // 8. Registry.
    let mut reg = PackRegistry::new();
    reg.register(pack.clone()).expect("register");
    let retrieved = reg
        .get("supply-chain", &pack.digest_urn)
        .expect("must retrieve compiled pack");
    assert_eq!(retrieved, &pack, "registry round-trip preserves bytes");

    // 9. Drift monitor consumes a runtime correction.
    let mut drift = DriftMonitor::new();
    let ok = drift.record(&Outcome {
        context_urn: "urn:blake3:dock-obstruction-confirmed".into(),
        deployed: AutonomicInstinct::Inspect,
        corrected: AutonomicInstinct::Inspect,
    });
    assert!(ok);
    assert!(!drift.drift_detected(1));
    drift.record(&Outcome {
        context_urn: "urn:blake3:dock-obstruction-confirmed".into(),
        deployed: AutonomicInstinct::Inspect,
        corrected: AutonomicInstinct::Escalate,
    });
    assert!(drift.drift_detected(1), "one mismatch crosses threshold 1");
}

#[test]
fn e2e_constant_policy_is_rejected_by_gauntlet() {
    // Anti-stub: a constant policy passes positive but fails perturbation.
    use autoinstinct::synth::CandidatePolicy;
    let policy = CandidatePolicy {
        rules: vec![
            ("urn:blake3:a".into(), AutonomicInstinct::Ask),
            ("urn:blake3:b".into(), AutonomicInstinct::Ask),
        ],
        default: AutonomicInstinct::Ask,
    };
    let scenarios = vec![JtbdScenario {
        name: "perturb".into(),
        context_urn: "urn:blake3:a".into(),
        expected: AutonomicInstinct::Ask,
        perturbed_context_urn: "urn:blake3:b".into(),
        forbidden: vec![],
    }];
    let report = gauntlet::run(&policy, &scenarios);
    assert!(!report.admitted());
    assert!(report
        .counterexamples
        .iter()
        .any(|c| c.surface == "perturbation"));
}
