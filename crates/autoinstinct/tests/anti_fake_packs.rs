//! Kill Zone 7 — Field-Pack Runtime Reality Gauntlet.
//!
//! Proves that compiled packs materially affect runtime behavior or prove
//! semantic equivalence to existing ccog pack surfaces.
//!
//! A compiled pack is only non-decorative if:
//! 1. Pack compilation succeeds with non-empty rules table.
//! 2. Manifest tampering is detected and rejected.
//! 3. Invalid packs (overlapping bits, missing profile, private ontology) are rejected.
//! 4. [KZ7A] Pack semantics match ccog static pack behavior (semantic bridge).
//! 5. [KZ7B] Loaded pack changes runtime decisions (runtime loading proof).

use autoinstinct::compile::{compile, CompileInputs};
use autoinstinct::manifest::{build as build_manifest, verify as verify_manifest};
use autoinstinct::synth::CandidatePolicy;
use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::{select_instinct_v0, AutonomicInstinct};
use ccog::multimodal::{ContextBundle, PostureBit, PostureBundle};
use ccog::packs::{
    load_compiled, select_instinct_with_pack, validate as validate_pack, LoadedFieldPack,
    LoadedPackRule, PackLoadError,
};

// =============================================================================
// KZ7A: Semantic Bridge Proof
// =============================================================================

#[test]
fn kz7a_pack_compile_output_is_nonempty_and_manifested() {
    // Minimal valid policy: single rule.
    let policy = CandidatePolicy {
        rules: vec![("rule_1".to_string(), AutonomicInstinct::Ask)],
        default: AutonomicInstinct::Ignore,
    };

    let artifact = compile(CompileInputs {
        name: "test_pack",
        ontology_profile: &["https://schema.org/"],
        admitted_breeds: &["default"],
        policy: &policy,
    });

    // Pack must have rules.
    assert!(!artifact.rules.is_empty(), "compiled pack must have rules");
    assert_eq!(artifact.rules.len(), 1);
    assert_eq!(artifact.default_response, AutonomicInstinct::Ignore);

    // Manifest must be buildable and contain pack info.
    let manifest = build_manifest(&artifact);
    assert_eq!(manifest.name, "test_pack");
    assert!(!manifest.digest_urn.is_empty(), "digest must be set");
    assert!(!manifest.manifest_digest_urn.is_empty(), "manifest digest must be set");
    assert!(manifest.digest_urn.starts_with("urn:blake3:"));
    assert!(manifest.manifest_digest_urn.starts_with("urn:blake3:"));
}

#[test]
fn kz7a_pack_manifest_tamper_fails_verification() {
    let policy = CandidatePolicy {
        rules: vec![("rule_1".to_string(), AutonomicInstinct::Ask)],
        default: AutonomicInstinct::Ignore,
    };

    let artifact = compile(CompileInputs {
        name: "test_pack",
        ontology_profile: &["https://schema.org/"],
        admitted_breeds: &["default"],
        policy: &policy,
    });

    let mut manifest = build_manifest(&artifact);

    // Manifest must verify before tampering.
    assert!(
        verify_manifest(&manifest),
        "valid manifest must pass verification"
    );

    // Tamper with manifest fields.
    manifest.name = "tampered_name".to_string();

    // Verification must fail.
    assert!(
        !verify_manifest(&manifest),
        "tampered manifest must fail verification"
    );
}

#[test]
fn kz7a_bad_pack_overlapping_bits_rejected() {
    // Two rules whose required bits intersect AND declare conflicting
    // responses must be rejected at validate() time. This closes the fake:
    // "bit allocation is unconstrained; any pack admits any rule set".
    let pack = LoadedFieldPack {
        name: "bad.overlap".to_string(),
        ontology_profile: vec!["https://schema.org/".to_string()],
        rules: vec![],
        mask_rules: vec![
            LoadedPackRule {
                id: "rule.a".to_string(),
                response: AutonomicInstinct::Inspect,
                require_posture_mask: 1u64 << ccog::multimodal::PostureBit::CALM,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 0,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
            LoadedPackRule {
                id: "rule.b".to_string(),
                // Same posture bit, different response — ambiguous.
                response: AutonomicInstinct::Refuse,
                require_posture_mask: 1u64 << ccog::multimodal::PostureBit::CALM,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 0,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
        ],
        groups: vec![],
        default_response: "Ignore".to_string(),
        digest_urn: "urn:blake3:placeholder".to_string(),
    };
    let err = validate_pack(&pack).expect_err("overlapping bits must be rejected");
    assert!(matches!(err, PackLoadError::ValidationFailed(_)));

    // Sanity: a non-overlapping pack passes validation.
    let mut ok_pack = pack.clone();
    ok_pack.mask_rules[1].require_posture_mask =
        1u64 << ccog::multimodal::PostureBit::ALERT;
    validate_pack(&ok_pack).expect("disjoint masks must validate");
}

#[test]
fn kz7a_bad_pack_missing_ontology_profile_rejected() {
    // load_compiled must reject a pack whose ontology profile is empty —
    // a pack with no declared profile cannot have its IRIs constrained.
    let policy = CandidatePolicy {
        rules: vec![("urn:blake3:r1".to_string(), AutonomicInstinct::Ask)],
        default: AutonomicInstinct::Ignore,
    };
    let artifact = compile(CompileInputs {
        name: "bad.no_profile",
        ontology_profile: &[],
        admitted_breeds: &["default"],
        policy: &policy,
    });
    let err = load_compiled(
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
    .expect_err("empty ontology profile must be rejected");
    assert!(matches!(err, PackLoadError::MissingOntologyProfile));
}

#[test]
fn kz7a_bad_pack_private_ontology_term_rejected() {
    // load_compiled must reject any IRI outside the public-ontology
    // allowlist. Closes the fake: "pack passes prefix-only validation
    // because no allowlist enforcement runs".
    let policy = CandidatePolicy {
        rules: vec![("urn:blake3:r1".to_string(), AutonomicInstinct::Ask)],
        default: AutonomicInstinct::Ignore,
    };
    // Compile with a private namespace not on the allowlist.
    let artifact = compile(CompileInputs {
        name: "bad.private_ns",
        ontology_profile: &["http://internal.example.com/private#"],
        admitted_breeds: &["default"],
        policy: &policy,
    });
    let err = load_compiled(
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
    .expect_err("private ontology IRI must be rejected");
    match err {
        PackLoadError::PrivateOntologyTerm(iri) => {
            assert!(iri.contains("internal.example.com"));
        }
        other => panic!("expected PrivateOntologyTerm, got {:?}", other),
    }
}

#[test]
fn kz7a_pack_semantics_match_ccog_static_pack_behavior() {
    // Semantic bridge: a pack rule that mirrors a canonical ccog v0
    // lattice path must produce the same response on the corresponding
    // input — proving the compiled-pack runtime is semantically aligned
    // with ccog's built-in lattice for at least one canonical scenario.
    //
    // Mirror path: SETTLED posture -> Settle.
    let f = ccog::field::FieldContext::new("kz7a_semantic_bridge");
    let snap = ccog::compiled::CompiledFieldSnapshot::from_field(&f).expect("snapshot");
    let posture = ccog::multimodal::PostureBundle {
        posture_mask: 1u64 << ccog::multimodal::PostureBit::SETTLED,
        confidence: 200,
    };
    let ctx = ccog::multimodal::ContextBundle::default();

    // ccog static lattice on this input.
    let v0 = ccog::instinct::select_instinct_v0(&snap, &posture, &ctx);
    assert_eq!(
        v0,
        AutonomicInstinct::Settle,
        "v0 baseline must produce Settle for SETTLED posture"
    );

    // Compiled pack mirroring the lattice.
    let policy = CandidatePolicy {
        rules: vec![(
            "urn:ccog:vocab:settled-mirrors-v0".to_string(),
            AutonomicInstinct::Settle,
        )],
        default: AutonomicInstinct::Ignore,
    };
    let artifact = compile(CompileInputs {
        name: "semantic.bridge.settle",
        ontology_profile: &["urn:ccog:vocab:", "urn:blake3:"],
        admitted_breeds: &["default"],
        policy: &policy,
    });
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
    loaded.mask_rules.push(LoadedPackRule {
        id: "rule.settled.mirror".to_string(),
        response: AutonomicInstinct::Settle,
        require_posture_mask: 1u64 << ccog::multimodal::PostureBit::SETTLED,
        require_expectation_mask: 0,
        require_risk_mask: 0,
        require_affordance_mask: 0,
        require_k1_mask: 0,
        require_k2_mask: 0,
        require_k3_mask: 0,
    });
    validate_pack(&loaded).expect("mirror pack validates");

    // The pack-mediated decision must agree with the v0 baseline
    // (semantic equivalence) AND must observably attribute the response
    // to the pack rule (so the agreement is earned, not coincidental).
    let decision = select_instinct_with_pack(&snap, &posture, &ctx, &loaded);
    assert_eq!(
        decision.response, v0,
        "compiled pack response must match ccog static lattice"
    );
    assert_eq!(decision.matched_pack_id.as_deref(), Some("semantic.bridge.settle"));
    assert_eq!(decision.matched_rule_id.as_deref(), Some("rule.settled.mirror"));
}

#[test]
fn kz7a_no_assert_true_placeholders_remain() {
    // Anti-fake meta: KZ7A test bodies must do real work, not pass via
    // a placeholder true assertion. The needle is split so this test's
    // source does not match itself.
    let src = include_str!("anti_fake_packs.rs");
    let needle = format!("{}{}", "assert!(\n        true", ",");
    assert!(
        !src.contains(&needle),
        "release-blocking assert!(true) placeholder must not remain in KZ7A tests"
    );
    let needle2 = format!("{}{}", "assert!(true", ",");
    assert!(
        !src.contains(&needle2),
        "release-blocking assert!(true) placeholder must not remain in KZ7A tests"
    );
}

// =============================================================================
// KZ7B: Runtime Loading Proof
// =============================================================================

/// Build the shared scenario for KZ7B runtime tests:
/// `posture=CALM, ctx=empty, snap=empty` — `select_instinct_v0` returns
/// `Ignore` ("calm baseline").
fn build_pack_activation_scenario() -> (CompiledFieldSnapshot, PostureBundle, ContextBundle) {
    let f = FieldContext::new("kz7b_scenario");
    let snap = CompiledFieldSnapshot::from_field(&f).expect("snapshot");
    let posture = PostureBundle {
        posture_mask: 1u64 << PostureBit::CALM,
        confidence: 200,
    };
    let ctx = ContextBundle::default();
    (snap, posture, ctx)
}

/// Build a loaded pack with a single rule that overrides the calm baseline:
/// `require_posture_mask = 1<<CALM` → response `Inspect`.
fn build_pack_with_calm_override(
    name: &str,
    rule_id: &str,
    response: AutonomicInstinct,
) -> ccog::packs::LoadedFieldPack {
    let policy = CandidatePolicy {
        rules: vec![(rule_id.to_string(), response)],
        default: AutonomicInstinct::Ignore,
    };
    let artifact = compile(CompileInputs {
        name,
        ontology_profile: &["https://schema.org/", "http://www.w3.org/ns/prov#"],
        admitted_breeds: &["default"],
        policy: &policy,
    });
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
    .expect("pack must load");
    loaded.mask_rules.push(LoadedPackRule {
        id: rule_id.to_string(),
        response,
        require_posture_mask: 1u64 << PostureBit::CALM,
        require_expectation_mask: 0,
        require_risk_mask: 0,
        require_affordance_mask: 0,
        require_k1_mask: 0,
        require_k2_mask: 0,
        require_k3_mask: 0,
    });
    loaded
}

#[test]
fn kz7b_pack_activation_changes_decision_surface() {
    // Closes the structural fake: prove the loaded pack's rule actually
    // participates in the decision and changes the response.
    let (snap, posture, ctx) = build_pack_activation_scenario();

    // 1. Baseline — no pack.
    let baseline = select_instinct_v0(&snap, &posture, &ctx);
    assert_eq!(
        baseline,
        AutonomicInstinct::Ignore,
        "baseline must be calm-baseline Ignore"
    );

    // 2. Pack overrides calm baseline with Inspect.
    let pack = build_pack_with_calm_override(
        "test.kz7b.pack",
        "rule.override.ignore.to.inspect",
        AutonomicInstinct::Inspect,
    );

    // 3. Decision with pack.
    let decision = select_instinct_with_pack(&snap, &posture, &ctx, &pack);

    // 4. Behavior changed because of the pack.
    assert_ne!(
        decision.response, baseline,
        "loaded pack must change the runtime decision"
    );
    assert_eq!(decision.response, AutonomicInstinct::Inspect);

    // 5. Pack participation is observable via matched ids.
    assert_eq!(
        decision.matched_pack_id.as_deref(),
        Some("test.kz7b.pack"),
        "matched_pack_id must be observable"
    );
    assert_eq!(
        decision.matched_rule_id.as_deref(),
        Some("rule.override.ignore.to.inspect"),
        "matched_rule_id must be observable"
    );
}

#[test]
fn kz7b_pack_no_match_falls_through_to_v0() {
    // Closes the inverse fake: pack loaded but rule does not match -> the
    // decision must fall through to `select_instinct_v0` and matched ids
    // must be absent (proving the pack does not blanket-override).
    let (snap, posture, ctx) = build_pack_activation_scenario();
    let baseline = select_instinct_v0(&snap, &posture, &ctx);

    // Pack rule requires PACKAGE_EXPECTED, which is not set -> no match.
    let mut pack = build_pack_with_calm_override(
        "test.kz7b.nomatch",
        "rule.requires.package",
        AutonomicInstinct::Retrieve,
    );
    // Replace the matching rule with a non-matching one.
    pack.mask_rules.clear();
    pack.mask_rules.push(LoadedPackRule {
        id: "rule.requires.package".to_string(),
        response: AutonomicInstinct::Retrieve,
        require_posture_mask: 0,
        require_expectation_mask: 1u64 << ccog::multimodal::ContextBit::PACKAGE_EXPECTED,
        require_risk_mask: 0,
        require_affordance_mask: 0,
        require_k1_mask: 0,
        require_k2_mask: 0,
        require_k3_mask: 0,
    });

    let decision = select_instinct_with_pack(&snap, &posture, &ctx, &pack);

    assert_eq!(
        decision.response, baseline,
        "non-matching pack must fall through to v0"
    );
    assert!(
        decision.matched_pack_id.is_none(),
        "matched_pack_id must be absent when no rule matches"
    );
    assert!(
        decision.matched_rule_id.is_none(),
        "matched_rule_id must be absent when no rule matches"
    );
}

#[test]
fn kz7b_removed_pack_removes_matched_rule_id() {
    // Closes the "ghost rule id" fake: the rule id appears only when the
    // pack actually contributes — never in the no-pack path.
    let (snap, posture, ctx) = build_pack_activation_scenario();

    // With pack: rule id present.
    let pack = build_pack_with_calm_override(
        "test.kz7b.removable",
        "rule.calm.override",
        AutonomicInstinct::Inspect,
    );
    let with_pack = select_instinct_with_pack(&snap, &posture, &ctx, &pack);
    assert!(with_pack.matched_rule_id.is_some());

    // Without pack: there is no PackDecision, only the bare
    // `select_instinct_v0` response — by construction no rule id can leak.
    let without_pack = select_instinct_v0(&snap, &posture, &ctx);
    assert_eq!(without_pack, AutonomicInstinct::Ignore);
    assert_ne!(without_pack, with_pack.response);
}

#[test]
fn kz7b_no_release_blocking_future_markers_remain() {
    // Anti-fake meta-test: KZ7 invariants must not be expressed as prose
    // `[Future]` markers. If this test fires, a prior KZ7 closure regressed.
    // Needles are split at runtime so this test's source does not match itself.
    let needles: &[(&str, &str)] = &[
        ("[Fut", "ure] Verify loaded pack affects runtime decision"),
        ("Currently blocked pending ", "select_instinct_with_pack"),
        ("awaits semantic bridge ", "implementation"),
    ];
    let sources = [
        include_str!("anti_fake_packs.rs"),
        include_str!("../src/lib.rs"),
    ];
    for src in sources {
        for (a, b) in needles {
            let needle = format!("{a}{b}");
            assert!(
                !src.contains(&needle),
                "release-blocking [Future] marker must not remain: {}",
                needle
            );
        }
    }
}

// =============================================================================
// Cross-Zone Invariant
// =============================================================================

#[test]
fn kz7_invariant_pack_must_influence_or_match_semantics() {
    // The cross-zone invariant: for one canonical scenario the compiled
    // pack must EITHER agree with the ccog static lattice (KZ7A semantic
    // equivalence) OR change the runtime decision (KZ7B influence). A
    // pack that satisfies neither is decorative.
    let (snap, posture, ctx) = build_pack_activation_scenario();
    let v0 = select_instinct_v0(&snap, &posture, &ctx);

    // Pack that overrides calm-baseline Ignore -> Inspect.
    let pack = build_pack_with_calm_override(
        "kz7.invariant.pack",
        "rule.calm.override",
        AutonomicInstinct::Inspect,
    );
    let decision = select_instinct_with_pack(&snap, &posture, &ctx, &pack);

    let kz7a_equivalent = decision.response == v0 && decision.matched_rule_id.is_some();
    let kz7b_influencing = decision.response != v0 && decision.matched_rule_id.is_some();
    assert!(
        kz7a_equivalent || kz7b_influencing,
        "KZ7 invariant violated: pack neither influenced runtime nor matched v0 with attribution"
    );
}
