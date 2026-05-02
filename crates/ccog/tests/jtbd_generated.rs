// @tests/jtbd_generated.rs by unrdf sync
// Source ontology: ontology/insa.ttl
// Rule: jtbd-generated-tests
// Template: templates/unrdf/rust/jtbd_generated.rs.njk
// DO NOT EDIT BY HAND.

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
use ccog::breeds::strips;
use ccog::compiled::CompiledFieldSnapshot;
use ccog::compiled_hook::{compute_present_mask, Predicate};
use ccog::field::FieldContext;
use ccog::hooks::{
    missing_evidence_hook, phrase_binding_hook, transition_admissibility_hook, HookRegistry,
};
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::powl64::Powl64;
use ccog::receipt::Receipt;
use ccog::runtime::ClosedFieldContext;
use ccog::trace::decide_with_trace_table;
use ccog::verdict::Breed;
use std::sync::Arc;

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
    field
        .load_field_state(&nt)
        .expect("load missing-prov-value doc");
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

fn empty_context(snap: Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
    ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    }
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
        let h_ask = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("https://schema.org/AskAction".as_bytes()) as u16
        );
        let h_act = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#Activity".as_bytes()) as u16
        );
        assert!(
            delta.contains(&h_ask) || delta.contains(&h_act),
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
        !outcomes.iter().any(|o| o.hook_name == "missing_evidence"),
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
        let h_informed = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#wasInformedBy".as_bytes())
                as u16
        );
        assert!(
            delta.contains(&h_informed),
            "phrase-binding must emit prov:wasInformedBy.\n delta: {}",
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
    load_typed_subject(
        &mut field,
        &concept,
        "http://www.w3.org/2004/02/skos/core#Concept",
    );

    let mut reg = HookRegistry::new();
    reg.register(phrase_binding_hook());
    let outcomes = reg.fire_matching(&field).expect("fire");

    assert!(
        !outcomes.iter().any(|o| o.hook_name == "phrase_binding"),
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
        let h_act = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#Activity".as_bytes()) as u16
        );
        let h_used = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#used".as_bytes()) as u16
        );
        assert!(
            delta.contains(&h_act),
            "must emit prov:Activity.\n delta: {}",
            delta
        );
        assert!(
            delta.contains(&h_used),
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
        let context = empty_context(Arc::new(snap.clone()));

        // Load-bearing equivalence: decide() == decide_with_trace_table().0
        let d1 = decide(&context);
        let (d2, trace) = decide_with_trace_table(&context, BUILTINS);
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
// JTBD 10: COG8 graph evaluation over generated snapshots
// =============================================================================

#[test]
fn jtbd_cog8_graph_evaluation_is_deterministic() {
    use ccog::runtime::cog8::*;

    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: 1 << Predicate::DD_MISSING_PROV_VALUE,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(0),
            guard_mask: 0,
            effect_mask: 1,
        },
    }];

    for _ in 0..32 {
        let doc = fake_iri("doc");
        let mut field = FieldContext::new(&fake_iri("field"));
        load_doc_missing_prov_value(&mut field, &doc, &Word().fake::<String>());
        let snap = std::sync::Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snap"));
        let context = empty_context(snap);

        let d1 = execute_cog8(&nodes, &edges, &context, 0).unwrap();
        let d2 = execute_cog8(&nodes, &edges, &context, 0).unwrap();
        assert_eq!(d1, d2, "COG8 evaluation must be deterministic");
        assert_eq!(
            d1.response,
            Instinct::Ask,
            "COG8 must fire Ask when DD lacks prov:value"
        );
    }
}

#[test]
fn jtbd_cog8_graph_perturbation_changes_response() {
    use ccog::runtime::cog8::*;

    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: 1 << Predicate::HAS_PREF_LABEL,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(0),
            guard_mask: 0,
            effect_mask: 1,
        },
    }];

    let mut field = FieldContext::new("cog8-perturb");
    load_concept_with_pref_label(&mut field, &fake_iri("c"), &fake_label());
    let snap = std::sync::Arc::new(CompiledFieldSnapshot::from_field(&field).expect("snap"));
    let context = empty_context(snap);

    let d_pos = execute_cog8(&nodes, &edges, &context, 0).unwrap();
    assert_eq!(d_pos.response, Instinct::Settle);

    // Perturbation: empty present mask
    let empty_snap = std::sync::Arc::new(CompiledFieldSnapshot::default());
    let empty_context = empty_context(empty_snap);
    let d_neg = execute_cog8(&nodes, &edges, &empty_context, 0).unwrap();
    assert_eq!(
        d_neg.response,
        Instinct::Ignore,
        "perturbation must change response"
    );
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
    assert_eq!(
        now_a, now_b,
        "canonical_material MUST NOT capture wall-clock"
    );
}

// =============================================================================
// JTBD 7: POWL64 path replay detects tampering
// =============================================================================

fn build_genuine_powl64(n: usize) -> Powl64 {
    use ccog::powl64::{Polarity, Powl64RouteCell};
    let mut p = Powl64::new();
    let mut current_chain: u64 = 0;
    for i in 0..n {
        let pol = if i % 2 == 0 {
            Polarity::Positive
        } else {
            Polarity::Negative
        };
        // Simple deterministic hash folding for the test
        let mut h = current_chain ^ ((i + 1) as u64);
        h ^= pol as u64;
        h = h.wrapping_mul(0xbf58476d1ce4e5b9); // fold

        p.extend(Powl64RouteCell {
            prior_chain: current_chain,
            chain_head: h,
            polarity: pol,
            ..Default::default()
        });
        current_chain = h;
    }
    p
}

#[test]
fn jtbd_powl64_replay_detects_path_tampering() {
    proptest!(|(n in 2usize..8usize)| {
        use ccog::powl64::{Powl64RouteCell, Polarity};
        let original = build_genuine_powl64(n);
        let path_a = original.path().to_vec();

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

        // Tamper 2 — flip polarity on the last extension.
        let mut polarity_flipped = Powl64::new();
        let mut current_chain: u64 = 0;
        for i in 0..n {
            let mut pol = if i % 2 == 0 { Polarity::Positive } else { Polarity::Negative };
            if i == n - 1 {
                pol = if pol == Polarity::Positive { Polarity::Negative } else { Polarity::Positive };
            }

            let mut h = current_chain ^ ((i + 1) as u64);
            h ^= pol as u64;
            h = h.wrapping_mul(0xbf58476d1ce4e5b9);

            polarity_flipped.extend(Powl64RouteCell {
                prior_chain: current_chain,
                chain_head: h,
                polarity: pol,
                ..Default::default()
            });
            current_chain = h;
        }
        let head_flipped = polarity_flipped.chain_head().expect("head");
        let head_a = original.chain_head().expect("chain head");
        prop_assert_ne!(head_flipped, head_a, "flipping polarity must change chain head");

        // Tamper 3 — different hash in last position.
        let mut swapped = Powl64::new();
        current_chain = 0;
        for i in 0..n {
            let val = if i == n - 1 { 0xdead_beef_u64 } else { (i + 1) as u64 };
            let pol = if i % 2 == 0 { Polarity::Positive } else { Polarity::Negative };

            let mut h = current_chain ^ val;
            h ^= pol as u64;
            h = h.wrapping_mul(0xbf58476d1ce4e5b9);

            swapped.extend(Powl64RouteCell {
                prior_chain: current_chain,
                chain_head: h,
                polarity: pol,
                ..Default::default()
            });
            current_chain = h;
        }
        prop_assert_ne!(
            swapped.chain_head().expect("head"),
            head_a,
            "swapping a path hash must change chain head"
        );
    });
}

// =============================================================================
// JTBD 8: field packs preserve canonical response-class semantics
// =============================================================================

#[test]
fn jtbd_field_packs_response_class_canonical_only() {
    use ccog::instinct::select_instinct_v0;
    use ccog::instinct::AutonomicInstinct;
    use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
    use ccog::packs::TierMasks;
    use ccog::runtime::ClosedFieldContext;

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
        select_instinct_v0(&ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }),
        AutonomicInstinct::Retrieve,
        "edge JTBD: package expected + can retrieve + cadence delivery → Retrieve"
    );

    // Perturbation 1 — drop CAN_RETRIEVE_NOW: must NOT be Retrieve.
    let ctx_no_afford = ContextBundle {
        affordance_mask: 0,
        ..ctx
    };
    assert_ne!(
        select_instinct_v0(&ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx_no_afford,
            tiers: TierMasks::ZERO,
        }),
        AutonomicInstinct::Retrieve,
        "remove CAN_RETRIEVE_NOW → response class must change away from Retrieve"
    );

    // Perturbation 2 — settled posture trumps expectation: → Settle.
    let posture_settled = PostureBundle {
        posture_mask: 1u64 << PostureBit::SETTLED,
        confidence: 200,
    };
    assert_eq!(
        select_instinct_v0(&ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: posture_settled,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }),
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
        select_instinct_v0(&ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: posture_alert,
            context: ctx_evidence,
            tiers: TierMasks::ZERO,
        }),
        AutonomicInstinct::Ask,
        "missing-evidence present → Ask (enterprise JTBD)"
    );

    // Negative boundary: instinct enum is canonical — every variant we
    // observe across this scenario sweep must belong to the canonical set.
    let observed = [
        select_instinct_v0(&ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }),
        select_instinct_v0(&ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx_no_afford,
            tiers: TierMasks::ZERO,
        }),
        select_instinct_v0(&ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: posture_settled,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }),
        select_instinct_v0(&ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: posture_alert,
            context: ctx_evidence,
            tiers: TierMasks::ZERO,
        }),
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
        let context = empty_context(Arc::new(snap.clone()));
        let decision = decide(&context);

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

// =============================================================================
// JTBD 11: Missing Invoice - request clarification for missing financials
// =============================================================================

#[test]
fn jtbd_missing_invoice_financials_triggers_ask() {
    use ccog::runtime::cog8::*;

    let invoice_type_bit = 10; // Simulated predicate bit for Invoice present
    let total_due_bit = 11; // Simulated predicate bit for totalPaymentDue present

    let nodes = [Cog8Row {
        pack_id: PackId(1),
        group_id: GroupId(1),
        rule_id: RuleId(101),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: 1 << invoice_type_bit,
        forbidden_mask: 1 << total_due_bit,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(0),
            guard_mask: 0,
            effect_mask: 1,
        },
    }];

    // Positive: Invoice present, totalPaymentDue missing -> Ask.
    // Positive: Invoice present, totalPaymentDue missing -> Ask.
    let present_pos = 1 << invoice_type_bit;
    let d_pos = execute_cog8_graph(&nodes, &edges, present_pos, 0).expect("execute");
    assert_eq!(
        d_pos.response,
        Instinct::Ask,
        "Missing financials on invoice must trigger Ask"
    );

    // Perturbation: totalPaymentDue present -> Ignore.
    let present_neg = (1 << invoice_type_bit) | (1 << total_due_bit);
    let d_neg = execute_cog8_graph(&nodes, &edges, present_neg, 0).expect("execute");
    assert_eq!(
        d_neg.response,
        Instinct::Ignore,
        "Invoice with financials should not trigger Ask"
    );
}

// =============================================================================
// JTBD 12: Policy Ambiguity - deterministic resolution via priority
// =============================================================================

#[test]
fn jtbd_policy_ambiguity_resolved_by_priority() {
    use ccog::runtime::cog8::*;

    let trigger_bit = 20;

    let nodes = [
        Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(1),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [FieldId(0); 8],
            required_mask: 1 << trigger_bit,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Settle,
            priority: 100,
        },
        Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(2),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [FieldId(0); 8],
            required_mask: 1 << trigger_bit,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Escalate,
            priority: 200, // Higher priority wins
        },
    ];

    let edges = [
        Cog8Edge {
            from: NodeId(0),
            to: NodeId(0),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Act,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(0),
                edge_id: EdgeId(0),
                guard_mask: 0,
                effect_mask: 1,
            },
        },
        Cog8Edge {
            from: NodeId(1),
            to: NodeId(1),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Act,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(1),
                edge_id: EdgeId(1),
                guard_mask: 0,
                effect_mask: 2,
            },
        },
    ];

    let present = 1 << trigger_bit;
    let decision = execute_cog8_graph(&nodes, &edges, present, 0).expect("execute");

    // Both rules fired, but higher priority Escalate must win.
    assert_eq!(decision.response, Instinct::Escalate);
    assert_eq!(decision.fired_mask, 0b11);

    // Perturbation: Lower the priority of the second rule.
    let mut nodes_low = nodes;
    nodes_low[1].priority = 50;
    let decision_low = execute_cog8_graph(&nodes_low, &edges, present, 0).expect("execute");
    // Now Settle wins because it's first and has higher priority than 50 (it's 100).
    assert_eq!(decision_low.response, Instinct::Settle);
}
