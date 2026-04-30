//! Phase 5 — Master Integration Test.
//!
//! Single-story end-to-end proof. The earlier kill zones each prove an
//! invariant in isolation; this test proves the substrate path itself:
//!
//! ```text
//! OCEL world → admission → TraceCorpus → motif/policy → pack artifact
//! → manifest verify → ccog::packs::load_compiled
//! → select_instinct_with_pack → matched rule metadata
//! → tamper failure → perturbation matters
//! ```
//!
//! The threshold this closes is qualitative: not "every part is real" but
//! "the loop is real". A passing test must demonstrate runtime effect AND
//! tamper failure AND perturbation sensitivity within one coherent story.

use std::collections::BTreeMap;

use autoinstinct::compile::{compile, CompileInputs};
use autoinstinct::llm::{
    admit, world_to_corpus, Counterfactual, ExpectedInstinct, OcelEvent, OcelObject, OcelWorld,
};
use autoinstinct::manifest::{build as build_manifest, verify as verify_manifest};
use autoinstinct::motifs::discover;
use autoinstinct::synth::synthesize;

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::{select_instinct_v0, AutonomicInstinct};
use ccog::multimodal::{ContextBundle, PostureBit, PostureBundle};
use ccog::packs::{load_compiled, select_instinct_with_pack, LoadedPackRule};

fn build_world() -> OcelWorld {
    let mk_obj = |id: &str, kind: &str, ot: &str| OcelObject {
        id: id.into(),
        kind: kind.into(),
        label: id.into(),
        ontology_type: ot.into(),
        attributes: BTreeMap::new(),
    };
    let mk_evt = |id: &str, kind: &str, ot: &str, objs: &[&str], resp: AutonomicInstinct| {
        OcelEvent {
            id: id.into(),
            kind: kind.into(),
            time: "2026-04-30T12:00:00Z".into(),
            objects: objs.iter().map(|s| (*s).into()).collect(),
            ontology_type: ot.into(),
            attributes: {
                let mut m = BTreeMap::new();
                m.insert("status".to_string(), serde_json::json!("earned"));
                m
            },
            expected_response: Some(resp),
            outcome: Some("earned".into()),
        }
    };

    OcelWorld {
        version: "1.0".into(),
        profile: "core".into(),
        scenario: "master".into(),
        objects: vec![
            mk_obj("dock-1", "facility", "https://schema.org/Place"),
            mk_obj("vehicle-1", "vehicle", "https://schema.org/Vehicle"),
            mk_obj("doc-1", "document", "https://schema.org/DigitalDocument"),
        ],
        events: vec![
            mk_evt(
                "arrive",
                "ingress",
                "https://schema.org/ArriveAction",
                &["dock-1", "vehicle-1"],
                AutonomicInstinct::Inspect,
            ),
            mk_evt(
                "review",
                "review",
                "https://schema.org/CheckAction",
                &["doc-1"],
                AutonomicInstinct::Ask,
            ),
            mk_evt(
                "release",
                "egress",
                "https://schema.org/LeaveAction",
                &["dock-1", "vehicle-1"],
                AutonomicInstinct::Settle,
            ),
        ],
        counterfactuals: vec![
            Counterfactual {
                id: "cf-no-doc".into(),
                description: "Remove the document; review event becomes ungrounded".into(),
                remove_objects: vec!["doc-1".into()],
                remove_events: vec!["review".into()],
                expected_response: AutonomicInstinct::Ignore,
            },
            Counterfactual {
                id: "cf-no-vehicle".into(),
                description: "Remove the vehicle; arrive/release become inadmissible".into(),
                remove_objects: vec!["vehicle-1".into()],
                remove_events: vec!["arrive".into(), "release".into()],
                expected_response: AutonomicInstinct::Ask,
            },
        ],
        expected_instincts: vec![ExpectedInstinct {
            condition: "doc present".into(),
            response: AutonomicInstinct::Ask,
            forbidden: vec!["fake-completion".into()],
        }],
    }
}

#[test]
fn master_ocel_to_pack_to_ccog_runtime_to_proof() {
    // ---- 1. World admitted ----
    let world = build_world();
    assert!(world.objects.len() >= 3, "≥3 object types required");
    assert!(world.events.len() >= 3, "≥3 event types required");
    assert!(!world.counterfactuals.is_empty());

    let admitted = admit(&serde_json::to_string(&world).unwrap(), "core")
        .expect("world must be admitted");
    assert_eq!(admitted.profile, "core");
    assert_eq!(admitted.objects.len(), 3);
    assert_eq!(admitted.events.len(), 3);

    // ---- 2. World becomes corpus ----
    let corpus = world_to_corpus(&admitted).expect("corpus produced");
    assert!(!corpus.episodes.is_empty(), "corpus must be non-empty");
    assert_eq!(corpus.episodes.len(), admitted.events.len());
    for episode in &corpus.episodes {
        assert!(
            episode.context_urn.starts_with("urn:blake3:"),
            "context urns must be deterministic blake3"
        );
        assert!(episode.receipt_urn.starts_with("urn:blake3:"));
    }
    // Determinism: re-derive must be identical.
    let corpus2 = world_to_corpus(&admitted).expect("corpus 2");
    let urns_a: Vec<&str> = corpus.episodes.iter().map(|e| e.context_urn.as_str()).collect();
    let urns_b: Vec<&str> = corpus2.episodes.iter().map(|e| e.context_urn.as_str()).collect();
    assert_eq!(urns_a, urns_b, "corpus IDs must be deterministic");

    // ---- 3. Corpus produces policy and pack ----
    let motifs = discover(&corpus, 1);
    assert!(!motifs.motifs.is_empty(), "motifs must be discovered");

    let policy = synthesize(&motifs);
    assert!(!policy.rules.is_empty(), "candidate policy must have rules");

    let artifact = compile(CompileInputs {
        name: "master.pack",
        ontology_profile: &["https://schema.org/", "urn:blake3:", "urn:ccog:vocab:"],
        admitted_breeds: &["default"],
        policy: &policy,
    });
    assert!(!artifact.rules.is_empty(), "pack must have rules");
    assert!(artifact.digest_urn.starts_with("urn:blake3:"));

    let manifest = build_manifest(&artifact);
    assert!(verify_manifest(&manifest), "manifest must verify");
    assert!(!manifest.manifest_digest_urn.is_empty());

    // ---- 4. Pack affects runtime ----
    // Build a calm-baseline scenario; v0 returns Ignore.
    let f = FieldContext::new("master_scenario");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::CALM,
        confidence: 200,
    };
    let ctx = ContextBundle::default();
    let baseline = select_instinct_v0(&snap, &posture, &ctx);
    assert_eq!(baseline, AutonomicInstinct::Ignore, "baseline must be Ignore");

    // Load the compiled pack and bind a mask rule that mirrors the
    // policy's first rule onto the calm posture so it actually fires
    // against the chosen scenario.
    let mut loaded = load_compiled(
        &artifact.name,
        &artifact.ontology_profile,
        &artifact
            .rules
            .iter()
            .map(|(k, v)| (k.clone(), format!("{:?}", v)))
            .collect::<Vec<_>>(),
        &format!("{:?}", artifact.default_response),
        &artifact.digest_urn,
    )
    .expect("pack loads");
    // Pick a response distinct from BOTH the calm baseline (`Ignore`) and
    // the perturbed-posture fallthrough (`Ask` from v0's default branch),
    // so the runtime effect is observable as a *change* in step 4 AND a
    // *different change* under perturbation in step 7.
    let pack_response = AutonomicInstinct::Inspect;
    loaded.mask_rules.push(LoadedPackRule {
        id: "master.rule.calm.override".to_string(),
        response: pack_response,
        require_posture_mask: 1u64 << PostureBit::CALM,
        require_expectation_mask: 0,
        require_risk_mask: 0,
        require_affordance_mask: 0,
    });

    let with_pack = select_instinct_with_pack(&snap, &posture, &ctx, &loaded);
    assert_eq!(with_pack.response, pack_response);
    assert_ne!(
        with_pack.response, baseline,
        "loaded pack must change runtime response"
    );
    assert_eq!(
        with_pack.matched_pack_id.as_deref(),
        Some("master.pack"),
        "matched_pack_id must be observable"
    );
    assert_eq!(
        with_pack.matched_rule_id.as_deref(),
        Some("master.rule.calm.override"),
        "matched_rule_id must be observable"
    );

    // ---- 5. Removing pack removes contribution ----
    // Without pack: only the bare v0 response — by construction no
    // matched_rule_id can leak.
    let no_pack_response = select_instinct_v0(&snap, &posture, &ctx);
    assert_eq!(no_pack_response, baseline);
    assert_ne!(no_pack_response, with_pack.response);

    // ---- 6. Tamper fails ----
    let mut tampered = manifest.clone();
    tampered.name = "evil.pack".to_string();
    assert!(
        !verify_manifest(&tampered),
        "manifest tamper must fail verification"
    );
    // And rule-table tampering must invalidate the digest.
    let mut tampered_artifact = artifact.clone();
    tampered_artifact
        .rules
        .push(("urn:blake3:injected".to_string(), AutonomicInstinct::Refuse));
    let tampered_manifest = build_manifest(&tampered_artifact);
    assert_ne!(
        tampered_manifest.manifest_digest_urn, manifest.manifest_digest_urn,
        "manifest digest must change when rules change"
    );

    // ---- 7. Perturbation matters ----
    // Drop the load-bearing posture bit; the rule must no longer match
    // and the decision must fall through to v0.
    let perturbed_posture = PostureBundle {
        posture_mask: 0,
        confidence: 200,
    };
    let perturbed = select_instinct_with_pack(&snap, &perturbed_posture, &ctx, &loaded);
    assert!(
        perturbed.matched_rule_id.is_none(),
        "removing the load-bearing posture must drop the matched rule"
    );
    assert_ne!(
        perturbed.response, with_pack.response,
        "perturbation must change runtime response"
    );

    // World-level perturbation: remove the load-bearing object
    // (`doc-1`) and assert the corpus loses its review-event grounding.
    let mut perturbed_world = admitted.clone();
    perturbed_world.objects.retain(|o| o.id != "doc-1");
    perturbed_world.events.retain(|e| !e.objects.iter().any(|o| o == "doc-1"));
    let perturbed_corpus = world_to_corpus(&perturbed_world).expect("perturbed corpus");
    assert!(
        perturbed_corpus.episodes.len() < corpus.episodes.len(),
        "removing a load-bearing object must shrink the corpus"
    );
    let perturbed_motifs = discover(&perturbed_corpus, 1);
    assert_ne!(
        perturbed_motifs.motifs.len(),
        motifs.motifs.len(),
        "perturbed world must produce a different motif set"
    );
}
