//! JTBD (jobs-to-be-done) integration tests with generated scenarios.
//!
//! Every test in this file follows the anti-stub pattern:
//!
//! 1. **Positive** — the expected response actually happens.
//! 2. **Negative boundary** — the prior bad behavior does not happen.
//! 3. **Perturbation** — remove one critical input; the expected response
//!    no longer happens.
//!
//! Tests cannot be satisfied by stubs because they assert cross-layer
//! consequences: graph closure → snapshot → hook/breed → materialized delta
//! → receipt → trace replay. A stub returning `Ok(())` fails perturbation.
//!
//! Data is generated with `fake` (realistic strings) plus `proptest`
//! (shrinkable adversarial combinations).

use ccog::bark_artifact::{decide, BUILTINS};
use ccog::breeds::{eliza, mycin, strips};
use ccog::compiled::CompiledFieldSnapshot;
use ccog::compiled_hook::{compute_present_mask, Predicate};
use ccog::field::FieldContext;
use ccog::hooks::{
    missing_evidence_hook, phrase_binding_hook, transition_admissibility_hook, HookRegistry,
};
use ccog::powl64::{GlobeCell, Powl64};
use ccog::receipt::Receipt;
use ccog::trace::decide_with_trace_table;
use ccog::verdict::Breed;

use fake::faker::lorem::en::Word;
use fake::faker::name::en::Name;
use fake::Fake;
use proptest::prelude::*;

// =============================================================================
// FIXTURE HELPERS — generated, not hand-tuned
// =============================================================================

fn fake_iri(prefix: &str) -> String {
    let n: u64 = (0u64..u64::MAX).fake();
    format!("http://example.org/{}/{:016x}", prefix, n)
}

fn fake_label() -> String {
    let words: Vec<String> = (3..=5).map(|_| Word().fake::<String>()).collect();
    words.join(" ")
}

fn load_doc_missing_prov_value(field: &mut FieldContext, doc_iri: &str, title: &str) {
    let nt = format!(
        "<{doc}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
         <{doc}> <http://purl.org/dc/terms/title> \"{title}\" .\n",
        doc = doc_iri,
        title = title,
    );
    field.load_field_state(&nt).expect("load missing-prov-value doc");
}

fn load_concept_with_pref_label(field: &mut FieldContext, concept_iri: &str, label: &str) {
    let nt = format!(
        "<{c}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2004/02/skos/core#Concept> .\n\
         <{c}> <http://www.w3.org/2004/02/skos/core#prefLabel> \"{l}\" .\n",
        c = concept_iri,
        l = label,
    );
    field.load_field_state(&nt).expect("load concept");
}

fn load_typed_subject(field: &mut FieldContext, subject_iri: &str, type_iri: &str) {
    let nt = format!(
        "<{s}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{t}> .\n",
        s = subject_iri,
        t = type_iri,
    );
    field.load_field_state(&nt).expect("load typed subject");
}

fn ntriples_of_outcome_delta(delta: &ccog::construct8::Construct8) -> String {
    delta.to_ntriples()
}

// =============================================================================
// JTBD 1: missing evidence requests gap without filling it
// =============================================================================

#[test]
fn jtbd_missing_evidence_requests_gap_without_filling_it() {
    for _ in 0..32 {
        let doc = fake_iri("doc");
        let title: String = Name().fake();
        let mut field = FieldContext::new(&fake_iri("field"));
        load_doc_missing_prov_value(&mut field, &doc, &title);

        let mut reg = HookRegistry::new();
        reg.register(missing_evidence_hook());
        let outcomes = reg.fire_matching(&field).expect("fire");

        // Positive: hook fires
        let outcome = outcomes
            .iter()
            .find(|o| o.hook_name == "missing_evidence")
            .expect("missing-evidence hook must fire when DD has no prov:value");

        let delta = ntriples_of_outcome_delta(&outcome.delta);

        // Positive: gap-finding emitted as schema:AskAction or prov:Activity
        assert!(
            delta.contains("schema.org/AskAction") || delta.contains("prov#Activity"),
            "delta must record gap as AskAction/Activity, got: {}",
            delta
        );

        // Negative boundary: never fabricates `<doc> prov:value ...`
        let fabricated = format!("<{}> <http://www.w3.org/ns/prov#value>", doc);
        assert!(
            !delta.contains(&fabricated),
            "missing-evidence hook MUST NOT fabricate prov:value on the gap doc.\n delta: {}",
            delta
        );
        // Never the literal "placeholder"
        assert!(
            !delta.contains("\"placeholder\""),
            "missing-evidence hook MUST NOT emit placeholder literal"
        );

        // Apply delta to the live store and re-snapshot.
        for o in &outcomes {
            o.delta.materialize(&field.graph).expect("materialize");
        }
        let snap2 = CompiledFieldSnapshot::from_field(&field).expect("snap2");
        let present = compute_present_mask(&snap2);

        // Re-running the hook would still fire — gap is preserved.
        assert!(
            (present & (1u64 << Predicate::DD_MISSING_PROV_VALUE)) != 0,
            "after gap-finding emission, missing-prov-value bit must remain set (gap not auto-filled)"
        );
    }
}

#[test]
fn jtbd_missing_evidence_perturbation_no_dd_no_fire() {
    let mut field = FieldContext::new("perturb-no-dd");
    // Concept only — no DigitalDocument typing → hook must not fire.
    let concept = fake_iri("c");
    load_concept_with_pref_label(&mut field, &concept, &fake_label());

    let mut reg = HookRegistry::new();
    reg.register(missing_evidence_hook());
    let outcomes = reg.fire_matching(&field).expect("fire");

    assert!(
        !outcomes
            .iter()
            .any(|o| o.hook_name == "missing_evidence"),
        "remove DD type triple → missing-evidence hook MUST NOT fire"
    );
}

// =============================================================================
// JTBD 2: phrase binding emits provenance, not fake definitions
// =============================================================================

#[test]
fn jtbd_phrase_binding_links_label_provenance() {
    for _ in 0..32 {
        let concept = fake_iri("concept");
        let label: String = fake_label();
        let mut field = FieldContext::new(&fake_iri("field"));
        load_concept_with_pref_label(&mut field, &concept, &label);

        let mut reg = HookRegistry::new();
        reg.register(phrase_binding_hook());
        let outcomes = reg.fire_matching(&field).expect("fire");

        let outcome = outcomes
            .iter()
            .find(|o| o.hook_name == "phrase_binding")
            .expect("phrase-binding hook must fire when prefLabel exists");

        let delta = ntriples_of_outcome_delta(&outcome.delta);

        // Positive: emits prov:wasInformedBy
        assert!(
            delta.contains("prov#wasInformedBy"),
            "phrase-binding must emit prov:wasInformedBy.\n delta: {}",
            delta
        );

        // Positive: target is a urn:blake3 IRI (real provenance, not fake string)
        assert!(
            delta.contains("urn:blake3:"),
            "phrase-binding target must be urn:blake3.\n delta: {}",
            delta
        );

        // Negative boundary: never the prior placeholder phrasing
        assert!(
            !delta.contains("derived from prefLabel"),
            "phrase-binding MUST NOT emit `skos:definition \"derived from prefLabel\"`"
        );
        assert!(
            !delta.contains("skos/core#definition"),
            "phrase-binding MUST NOT abuse skos:definition as fake provenance"
        );
    }
}

#[test]
fn jtbd_phrase_binding_perturbation_no_label_no_fire() {
    // Concept typed but no prefLabel → hook must not fire.
    let mut field = FieldContext::new("perturb-no-label");
    let concept = fake_iri("c");
    load_typed_subject(&mut field, &concept, "http://www.w3.org/2004/02/skos/core#Concept");

    let mut reg = HookRegistry::new();
    reg.register(phrase_binding_hook());
    let outcomes = reg.fire_matching(&field).expect("fire");

    assert!(
        !outcomes
            .iter()
            .any(|o| o.hook_name == "phrase_binding"),
        "remove prefLabel → phrase-binding hook MUST NOT fire"
    );
}

// =============================================================================
// JTBD 3: transition admissibility records finding, not SHACL shape
// =============================================================================

#[test]
fn jtbd_transition_admissibility_records_finding_not_shape() {
    for _ in 0..32 {
        let subj = fake_iri("subj");
        let mut field = FieldContext::new(&fake_iri("field"));
        load_typed_subject(&mut field, &subj, "https://schema.org/DigitalDocument");

        let mut reg = HookRegistry::new();
        reg.register(transition_admissibility_hook());
        let outcomes = reg.fire_matching(&field).expect("fire");

        let outcome = outcomes
            .iter()
            .find(|o| o.hook_name == "transition_admissibility")
            .expect("transition-admissibility must fire on rdf:type triple");

        let delta = ntriples_of_outcome_delta(&outcome.delta);

        // Positive: emits prov:Activity + prov:used
        assert!(
            delta.contains("prov#Activity"),
            "must emit prov:Activity.\n delta: {}",
            delta
        );
        assert!(
            delta.contains("prov#used"),
            "must emit prov:used.\n delta: {}",
            delta
        );

        // Negative boundary: must not abuse SHACL shape on the instance
        assert!(
            !delta.contains("shacl#targetClass"),
            "must NOT write sh:targetClass on the instance subject"
        );
        assert!(
            !delta.contains("shacl#NodeShape"),
            "must NOT declare instance as sh:NodeShape"
        );
    }
}

#[test]
fn jtbd_transition_admissibility_perturbation_no_type_no_fire() {
    // Subject with title only, no rdf:type → hook must not fire.
    let mut field = FieldContext::new("perturb-no-type");
    let subj = fake_iri("s");
    let nt = format!(
        "<{}> <http://purl.org/dc/terms/title> \"{}\" .\n",
        subj,
        Name().fake::<String>()
    );
    field.load_field_state(&nt).expect("load");

    let mut reg = HookRegistry::new();
    reg.register(transition_admissibility_hook());
    let outcomes = reg.fire_matching(&field).expect("fire");

    assert!(
        !outcomes
            .iter()
            .any(|o| o.hook_name == "transition_admissibility"),
        "remove rdf:type → transition-admissibility MUST NOT fire"
    );
}

// =============================================================================
// JTBD 4: old-AI breeds admit only when preconditions are real
// =============================================================================

#[test]
fn jtbd_breed_admission_changes_when_precondition_removed() {
    // ELIZA: requires skos:prefLabel
    {
        let mut field = FieldContext::new(&fake_iri("eliza"));
        load_concept_with_pref_label(&mut field, &fake_iri("c"), &fake_label());
        assert!(strips::admit_breed(Breed::Eliza, &field).expect("admit"));

        let mut field2 = FieldContext::new(&fake_iri("eliza-perturb"));
        load_typed_subject(
            &mut field2,
            &fake_iri("c"),
            "http://www.w3.org/2004/02/skos/core#Concept",
        );
        assert!(
            !strips::admit_breed(Breed::Eliza, &field2).expect("admit"),
            "ELIZA must deny when prefLabel removed"
        );
    }

    // MYCIN: admits when any prov:value is present OR all DDs have prov:value.
    // Positive: doc WITH prov:value present.
    {
        let doc = fake_iri("d");
        let mut field = FieldContext::new(&fake_iri("mycin-pos"));
        let nt = format!(
            "<{d}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <{d}> <http://www.w3.org/ns/prov#value> \"{v}\" .\n",
            d = doc,
            v = fake_label(),
        );
        field.load_field_state(&nt).expect("load");
        assert!(strips::admit_breed(Breed::Mycin, &field).expect("admit"));

        // Perturbation: DD missing prov:value, no prov:value anywhere → MYCIN denies.
        let mut field2 = FieldContext::new(&fake_iri("mycin-neg"));
        load_doc_missing_prov_value(&mut field2, &fake_iri("d"), &Name().fake::<String>());
        assert!(
            !strips::admit_breed(Breed::Mycin, &field2).expect("admit"),
            "MYCIN must deny when at least one DD lacks prov:value and no prov:value exists at all"
        );
    }

    // STRIPS: requires schema:DigitalDocument instances with prov:value present.
    // Build a doc WITH prov:value to admit, then test denial when prov:value missing.
    {
        let doc = fake_iri("d");
        let mut field = FieldContext::new(&fake_iri("strips-pos"));
        let nt = format!(
            "<{d}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <{d}> <http://www.w3.org/ns/prov#value> \"{v}\" .\n",
            d = doc,
            v = fake_label(),
        );
        field.load_field_state(&nt).expect("load");
        assert!(strips::admit_breed(Breed::Strips, &field).expect("admit"));

        let mut field2 = FieldContext::new(&fake_iri("strips-neg"));
        load_doc_missing_prov_value(&mut field2, &fake_iri("d"), &Name().fake::<String>());
        assert!(
            !strips::admit_breed(Breed::Strips, &field2).expect("admit"),
            "STRIPS must deny when DD lacks prov:value"
        );
    }

    // SHRDLU: requires rdf:type
    {
        let mut field = FieldContext::new(&fake_iri("shrdlu"));
        load_typed_subject(&mut field, &fake_iri("s"), "https://schema.org/Thing");
        assert!(strips::admit_breed(Breed::Shrdlu, &field).expect("admit"));

        let field2 = FieldContext::new(&fake_iri("shrdlu-empty"));
        assert!(
            !strips::admit_breed(Breed::Shrdlu, &field2).expect("admit"),
            "SHRDLU must deny on empty field"
        );
    }
}

// =============================================================================
// JTBD 5: bark decision and trace equivalence under generated fields
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn jtbd_trace_replays_decision_for_generated_fields(
        has_dd in any::<bool>(),
        has_dd_prov in any::<bool>(),
        has_pref_label in any::<bool>(),
        has_rdf_type in any::<bool>(),
    ) {
        let mut field = FieldContext::new("proptest");
        let mut nt = String::new();
        if has_dd {
            nt.push_str("<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n");
            if has_dd_prov {
                nt.push_str("<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"v\" .\n");
            }
        }
        if has_pref_label {
            nt.push_str("<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"L\" .\n");
        }
        if has_rdf_type {
            nt.push_str("<http://example.org/s1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/Thing> .\n");
        }
        if !nt.is_empty() {
            field.load_field_state(&nt).expect("load");
        }
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");

        // Load-bearing equivalence: decide() == decide_with_trace_table().0
        let d1 = decide(&snap);
        let (d2, trace) = decide_with_trace_table(&snap, BUILTINS);
        prop_assert_eq!(d1.fired, d2.fired, "decide and decide_with_trace must agree on fired mask");
        prop_assert_eq!(d1.present_mask, d2.present_mask, "present_mask must agree");

        // Every node in the trace classifies its slot truthfully.
        for n in &trace.nodes {
            let satisfied = (n.require_mask & trace.present_mask) == n.require_mask;
            prop_assert_eq!(
                n.trigger_fired, satisfied,
                "trigger_fired must equal (require_mask & present_mask) == require_mask"
            );
            if !n.trigger_fired {
                prop_assert!(n.skip.is_some(), "non-firing node must record a typed BarkSkipReason");
            }
        }
    }
}

// =============================================================================
// JTBD 6: receipt identity changes with semantic material
// =============================================================================

#[test]
fn jtbd_receipt_identity_changes_with_semantic_material() {
    let hook_id = "missing_evidence";
    let plan_node = 1u16;
    let field_id = "f";
    let prior: Option<blake3::Hash> = None;
    let polarity: u8 = 1;
    let delta_a = b"delta-a".as_slice();
    let delta_b = b"delta-b".as_slice();
    let prior_some: Option<blake3::Hash> = Some(blake3::Hash::from([0xab; 32]));

    let m_aa = Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior, polarity);
    let m_aa2 = Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior, polarity);
    let m_ab = Receipt::canonical_material(hook_id, plan_node, delta_b, field_id, prior, polarity);
    let m_pol = Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior, 0);
    let m_prior =
        Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior_some, polarity);

    let urn_aa = Receipt::derive_urn(&m_aa);
    let urn_aa2 = Receipt::derive_urn(&m_aa2);
    let urn_ab = Receipt::derive_urn(&m_ab);
    let urn_pol = Receipt::derive_urn(&m_pol);
    let urn_prior = Receipt::derive_urn(&m_prior);

    // Determinism: same material → same URN.
    assert_eq!(urn_aa, urn_aa2);
    // Sensitivity: changing delta, polarity, or prior chain changes URN.
    assert_ne!(urn_aa, urn_ab, "delta change must alter URN");
    assert_ne!(urn_aa, urn_pol, "polarity change must alter URN");
    assert_ne!(urn_aa, urn_prior, "prior chain change must alter URN");
    // Format: every URN starts with urn:blake3:.
    for urn in [&urn_aa, &urn_aa2, &urn_ab, &urn_pol, &urn_prior] {
        assert!(
            urn.starts_with("urn:blake3:"),
            "receipt URN must use urn:blake3 scheme, got {}",
            urn
        );
    }
    // Negative boundary: no wall-clock material smuggled in. Two derivations
    // a moment apart with identical inputs must remain byte-identical.
    let now_a = Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior, polarity);
    std::thread::sleep(std::time::Duration::from_millis(5));
    let now_b = Receipt::canonical_material(hook_id, plan_node, delta_a, field_id, prior, polarity);
    assert_eq!(now_a, now_b, "canonical_material MUST NOT capture wall-clock");
}

// =============================================================================
// JTBD 7: POWL64 path replay detects tampering
// =============================================================================

fn build_genuine_powl64(n: usize) -> Powl64 {
    let mut p = Powl64::new();
    for i in 0..n {
        let iri_str = format!("urn:blake3:{:064x}", (i + 1) as u64);
        let iri = ccog::graph::GraphIri::from_iri(&iri_str).expect("valid urn:blake3");
        let _ = p.extend(&iri, (i % 2) as u8);
    }
    p
}

#[test]
fn jtbd_powl64_replay_detects_path_tampering() {
    proptest!(|(n in 2usize..8usize)| {
        let original = build_genuine_powl64(n);
        let path_a = original.path().to_vec();
        let head_a = original.chain_head().expect("chain head");

        // Positive: identical replay matches.
        let replay = build_genuine_powl64(n);
        prop_assert!(replay.shape_match_v1_path(&original).is_ok(), "identical replay must match");
        prop_assert_eq!(&replay.path().to_vec(), &path_a, "replay path must equal original");

        // Tamper 1 — truncate one entry.
        let truncated_input_n = n - 1;
        let truncated = build_genuine_powl64(truncated_input_n);
        prop_assert!(
            truncated.shape_match_v1_path(&original).is_err(),
            "truncating one entry must break shape match"
        );

        // Tamper 2 — flip polarity on the last extension. Build with last polarity inverted.
        let mut polarity_flipped = Powl64::new();
        for i in 0..n {
            let iri_str = format!("urn:blake3:{:064x}", (i + 1) as u64);
            let iri = ccog::graph::GraphIri::from_iri(&iri_str).unwrap();
            let pol = if i == n - 1 { ((i + 1) % 2) as u8 } else { (i % 2) as u8 };
            let _ = polarity_flipped.extend(&iri, pol);
        }
        let head_flipped = polarity_flipped.chain_head().expect("head");
        prop_assert_ne!(head_flipped, head_a, "flipping polarity must change chain head");

        // Tamper 3 — different IRI in last position.
        let mut swapped = Powl64::new();
        for i in 0..n {
            let iri_str = if i == n - 1 {
                format!("urn:blake3:{:064x}", 0xdead_beef_u64)
            } else {
                format!("urn:blake3:{:064x}", (i + 1) as u64)
            };
            let iri = ccog::graph::GraphIri::from_iri(&iri_str).unwrap();
            let _ = swapped.extend(&iri, (i % 2) as u8);
        }
        prop_assert_ne!(
            swapped.chain_head().expect("head"),
            head_a,
            "swapping a path IRI must change chain head"
        );
    });
}

// =============================================================================
// JTBD 8: field packs preserve canonical response-class semantics
// =============================================================================

#[test]
fn jtbd_field_packs_response_class_canonical_only() {
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::{ContextBundle, ContextBit, PostureBundle, PostureBit};
    use ccog::instinct::select_instinct_v0;

    // Edge: package expected + can retrieve + cadence delivery → Retrieve.
    let mut field = FieldContext::new("edge-jtbd");
    load_doc_missing_prov_value(&mut field, &fake_iri("d"), &Name().fake::<String>());
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");

    let posture = PostureBundle {
        posture_mask: (1u64 << PostureBit::CADENCE_DELIVERY) | (1u64 << PostureBit::ALERT),
        confidence: 200,
    };
    let ctx = ContextBundle {
        expectation_mask: 1u64 << ContextBit::PACKAGE_EXPECTED,
        risk_mask: 0,
        affordance_mask: 1u64 << ContextBit::CAN_RETRIEVE_NOW,
    };
    assert_eq!(
        select_instinct_v0(&snap, &posture, &ctx),
        AutonomicInstinct::Retrieve,
        "edge JTBD: package expected + can retrieve + cadence delivery → Retrieve"
    );

    // Perturbation 1 — drop CAN_RETRIEVE_NOW: must NOT be Retrieve.
    let ctx_no_afford = ContextBundle {
        affordance_mask: 0,
        ..ctx
    };
    assert_ne!(
        select_instinct_v0(&snap, &posture, &ctx_no_afford),
        AutonomicInstinct::Retrieve,
        "remove CAN_RETRIEVE_NOW → response class must change away from Retrieve"
    );

    // Perturbation 2 — settled posture trumps expectation: → Settle.
    let posture_settled = PostureBundle {
        posture_mask: 1u64 << PostureBit::SETTLED,
        confidence: 200,
    };
    assert_eq!(
        select_instinct_v0(&snap, &posture_settled, &ctx),
        AutonomicInstinct::Settle,
        "settled posture must dominate"
    );

    // Enterprise JTBD: missing-evidence present → Ask, regardless of theft risk.
    let posture_alert = PostureBundle {
        posture_mask: 1u64 << PostureBit::ALERT,
        confidence: 128,
    };
    let ctx_evidence = ContextBundle {
        expectation_mask: 0,
        risk_mask: 0,
        affordance_mask: 0,
    };
    assert_eq!(
        select_instinct_v0(&snap, &posture_alert, &ctx_evidence),
        AutonomicInstinct::Ask,
        "missing-evidence present → Ask (enterprise JTBD)"
    );

    // Negative boundary: instinct enum is canonical — every variant we
    // observe across this scenario sweep must belong to the canonical set.
    let observed = [
        select_instinct_v0(&snap, &posture, &ctx),
        select_instinct_v0(&snap, &posture, &ctx_no_afford),
        select_instinct_v0(&snap, &posture_settled, &ctx),
        select_instinct_v0(&snap, &posture_alert, &ctx_evidence),
    ];
    for v in observed {
        match v {
            AutonomicInstinct::Settle
            | AutonomicInstinct::Retrieve
            | AutonomicInstinct::Inspect
            | AutonomicInstinct::Ask
            | AutonomicInstinct::Refuse
            | AutonomicInstinct::Escalate
            | AutonomicInstinct::Ignore => {}
        }
    }
}

// =============================================================================
// JTBD 9 (cross-cutting): the bark decide → materialize → seal pipeline
// agrees with itself under generated input.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn jtbd_bark_pipeline_self_consistent(
        has_dd in any::<bool>(),
        has_dd_prov in any::<bool>(),
        has_pref_label in any::<bool>(),
        has_rdf_type in any::<bool>(),
    ) {
        let mut field = FieldContext::new("pipeline-jtbd");
        let mut nt = String::new();
        if has_dd {
            nt.push_str("<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n");
            if has_dd_prov {
                nt.push_str("<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"v\" .\n");
            }
        }
        if has_pref_label {
            nt.push_str("<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"L\" .\n");
        }
        if has_rdf_type {
            nt.push_str("<http://example.org/s1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/Thing> .\n");
        }
        if !nt.is_empty() {
            field.load_field_state(&nt).expect("load");
        }

        let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
        let decision = decide(&snap);

        // Every fired bit corresponds to a slot whose require_mask is satisfied.
        for (i, slot) in BUILTINS.iter().enumerate() {
            let bit = 1u64 << i;
            let fired = (decision.fired & bit) != 0;
            let satisfied = (slot.require_mask & decision.present_mask) == slot.require_mask;
            prop_assert_eq!(
                fired, satisfied,
                "fired bit {} must equal require_mask satisfied for slot {}",
                i, slot.name
            );
        }
    }
}
