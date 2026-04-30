//! Kill Zone 8 — Report Authenticity Gauntlet (Phase 6).
//!
//! Closes the fake path:
//!
//! ```text
//! Gemini wrote a beautiful summary, therefore the system is real.
//! ```
//!
//! KZ8 makes admission load-bearing: every claim must cite a real
//! evidence file and a substring snippet that is actually present in
//! that file; status and commit must align with the supplied
//! scorecard; forbidden over-claims (SOC2/HIPAA/FedRAMP/...) cause
//! rejection; kind-specific citations (master, KZ7 runtime, zero-alloc,
//! OCEL) are required when the corresponding triggers appear.

use std::collections::BTreeMap;
use std::path::PathBuf;

use autoinstinct::report::admit::{admit, ReportAdmissionError, FORBIDDEN_OVERCLAIMS};
use autoinstinct::report::diff;
use autoinstinct::report::evidence::{EvidenceBundle, REQUIRED_FILES};
use autoinstinct::report::schema::{
    GeneratedReport, OpenRisk, ReportClaim, ReportKind, ReportStatus, RiskSeverity,
};
use autoinstinct::scorecard::{all_true_scorecard, Scorecard};

// ============================================================================
// Bundle / report fixtures (synthetic — no live Gemini)
// ============================================================================

fn fixture_bundle(scorecard: Scorecard) -> EvidenceBundle {
    let mut outputs = BTreeMap::new();
    outputs.insert("scorecard.json".to_string(), scorecard.to_json().unwrap());
    outputs.insert("git.txt".to_string(), "branch: x\ncommit: abc\n".to_string());
    outputs.insert("toolchain.txt".to_string(), "rustc 1.0\n".to_string());
    outputs.insert(
        "anti_fake_doctrine.out".to_string(),
        "test doctrine_constants_are_used_by_runtime_paths ... ok\n".to_string(),
    );
    outputs.insert(
        "anti_fake_causal.out".to_string(),
        "test causal_every_perturbation_changes_response ... ok\n".to_string(),
    );
    outputs.insert(
        "anti_fake_ocel.out".to_string(),
        "test ocel_admitted_world_produces_nonempty_corpus ... ok\n\
         OCEL admission rejected the flat world.\n"
            .to_string(),
    );
    outputs.insert(
        "anti_fake_perf.out".to_string(),
        "test anti_fake_decide_is_zero_heap_and_input_dependent ... ok\n\
         alloc-free decide path measured.\n"
            .to_string(),
    );
    outputs.insert(
        "anti_fake_packs.out".to_string(),
        "test kz7b_pack_activation_changes_decision_surface ... ok\n\
         matched_rule_id observed in PackDecision.\n"
            .to_string(),
    );
    outputs.insert(
        "anti_fake_master.out".to_string(),
        "test master_ocel_to_pack_to_ccog_runtime_to_proof ... ok\n\
         master loop closed.\n"
            .to_string(),
    );
    outputs.insert(
        "ccog.out".to_string(),
        "test result: ok. 196 passed\n".to_string(),
    );
    outputs.insert(
        "autoinstinct.out".to_string(),
        "test result: ok. 118 passed\n".to_string(),
    );
    EvidenceBundle {
        root: PathBuf::from("/tmp/kz8-fixture"),
        scorecard: serde_json::from_str(outputs.get("scorecard.json").unwrap()).unwrap(),
        git_txt: outputs.get("git.txt").cloned().unwrap_or_default(),
        toolchain_txt: outputs.get("toolchain.txt").cloned().unwrap_or_default(),
        outputs,
    }
}

fn well_formed_report(commit: &str) -> GeneratedReport {
    GeneratedReport {
        report_kind: ReportKind::Executive,
        title: "Anti-Fake Substrate Closure".to_string(),
        commit: commit.to_string(),
        toolchain: "rustc 1.0".to_string(),
        overall_status: ReportStatus::Pass,
        claims: vec![
            ReportClaim {
                id: "kz7.runtime.pack.observable".to_string(),
                claim: "Loaded pack changes runtime decision and exposes matched_rule_id."
                    .to_string(),
                evidence_files: vec!["anti_fake_packs.out".to_string()],
                evidence_snippets: vec![
                    "kz7b_pack_activation_changes_decision_surface".to_string(),
                    "matched_rule_id".to_string(),
                ],
                risk_if_false: "Pack layer would be decorative.".to_string(),
            },
            ReportClaim {
                id: "master.loop.closed".to_string(),
                claim: "The master loop from world generation to ccog runtime decision is proven."
                    .to_string(),
                evidence_files: vec!["anti_fake_master.out".to_string()],
                evidence_snippets: vec![
                    "master_ocel_to_pack_to_ccog_runtime_to_proof".to_string(),
                ],
                risk_if_false: "Composition between subsystems would be unproven.".to_string(),
            },
        ],
        open_risks: vec![OpenRisk {
            id: "ent.ops.hardening".to_string(),
            risk: "Enterprise operational hardening remains.".to_string(),
            severity: RiskSeverity::P1,
            mitigation: "Track signed pack distribution and tenant isolation as follow-ups."
                .to_string(),
        }],
        markdown: "# Anti-Fake Substrate Closure\n\nThe master loop is proven.\n".to_string(),
    }
}

fn bundle_commit(b: &EvidenceBundle) -> String {
    b.scorecard.commit_recorded.clone()
}

// ============================================================================
// KZ8 admission gauntlet
// ============================================================================

#[test]
fn kz8_report_admits_well_formed_report() {
    // Positive control — admission must accept a fully well-formed report.
    let mut card = all_true_scorecard();
    card.commit_recorded = "f497664".to_string();
    let bundle = fixture_bundle(card);
    let report = well_formed_report(&bundle_commit(&bundle));
    admit(&report, &bundle).expect("well-formed report must admit");
}

#[test]
fn kz8_report_rejects_pass_when_scorecard_fails() {
    let mut card = all_true_scorecard();
    card.kz7_runtime_loading_pass = false;
    card.recompute_overall();
    card.commit_recorded = "deadbeef".into();
    assert!(!card.overall_pass);
    let bundle = fixture_bundle(card);

    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.overall_status = ReportStatus::Pass; // lying

    match admit(&report, &bundle) {
        Err(ReportAdmissionError::StatusMismatch { .. }) => {}
        other => panic!("expected StatusMismatch, got {other:?}"),
    }
}

#[test]
fn kz8_report_rejects_commit_mismatch() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "real-commit".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.commit = "other-commit".into();
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::CommitMismatch { .. }) => {}
        other => panic!("expected CommitMismatch, got {other:?}"),
    }
}

#[test]
fn kz8_report_rejects_unknown_evidence_file() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.claims[0].evidence_files = vec!["nonexistent.out".into()];
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::EvidenceFileMissing { file, .. }) => {
            assert_eq!(file, "nonexistent.out");
        }
        other => panic!("expected EvidenceFileMissing, got {other:?}"),
    }
}

#[test]
fn kz8_report_rejects_missing_snippet() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.claims[0].evidence_snippets = vec!["completely-fabricated-string".into()];
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::SnippetNotFound { snippet, .. }) => {
            assert_eq!(snippet, "completely-fabricated-string");
        }
        other => panic!("expected SnippetNotFound, got {other:?}"),
    }
}

#[test]
fn kz8_report_rejects_empty_citation_files() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.claims[0].evidence_files.clear();
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::EmptyCitation { what, .. }) => {
            assert_eq!(what, "evidence_files");
        }
        other => panic!("expected EmptyCitation(evidence_files), got {other:?}"),
    }
}

#[test]
fn kz8_report_rejects_overclaim_in_markdown() {
    // One sub-assertion per forbidden term — every term must be detected.
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    for term in FORBIDDEN_OVERCLAIMS {
        let mut report = well_formed_report(&bundle_commit(&bundle));
        report.markdown = format!("# Pitch\n\nThis system is {term} today.\n");
        match admit(&report, &bundle) {
            Err(ReportAdmissionError::Overclaim(t)) => {
                assert_eq!(t.to_ascii_lowercase(), term.to_ascii_lowercase());
            }
            other => panic!("expected Overclaim({term}), got {other:?}"),
        }
    }
}

#[test]
fn kz8_report_rejects_overclaim_in_open_risks() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.open_risks[0].mitigation = "We are SOC2 certified next quarter.".into();
    assert!(matches!(
        admit(&report, &bundle),
        Err(ReportAdmissionError::Overclaim(_))
    ));
}

#[test]
fn kz8_report_requires_master_claim_to_cite_master_output() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    // Strip the master citation while keeping master language in the claim.
    report.claims[1].evidence_files = vec!["anti_fake_packs.out".into()];
    report.claims[1].evidence_snippets = vec!["matched_rule_id".into()];
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::MissingRequiredCitation { required, .. }) => {
            assert_eq!(required, "anti_fake_master.out");
        }
        other => panic!("expected MissingRequiredCitation(master), got {other:?}"),
    }
}

#[test]
fn kz8_report_requires_kz7_claim_to_cite_packs_output() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    // Drop the packs citation but keep KZ7 language ("matched_rule_id").
    report.claims[0].evidence_files = vec!["anti_fake_doctrine.out".into()];
    report.claims[0].evidence_snippets = vec!["doctrine_constants_are_used_by_runtime_paths".into()];
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::MissingRequiredCitation { required, .. }) => {
            assert_eq!(required, "anti_fake_packs.out");
        }
        other => panic!("expected MissingRequiredCitation(packs), got {other:?}"),
    }
}

#[test]
fn kz8_report_requires_zero_alloc_claim_to_cite_perf_output() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    // Replace the second claim with a zero-alloc claim that doesn't cite perf.
    report.claims[1] = ReportClaim {
        id: "perf.zero.alloc".into(),
        claim: "The hot path is alloc-free under load.".into(),
        evidence_files: vec!["anti_fake_doctrine.out".into()],
        evidence_snippets: vec!["doctrine_constants_are_used_by_runtime_paths".into()],
        risk_if_false: "Latency claims would be unfounded.".into(),
    };
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::MissingRequiredCitation { required, .. }) => {
            assert_eq!(required, "anti_fake_perf.out");
        }
        other => panic!("expected MissingRequiredCitation(perf), got {other:?}"),
    }
}

#[test]
fn kz8_report_requires_ocel_claim_to_cite_ocel_output() {
    let mut card = all_true_scorecard();
    card.commit_recorded = "c".into();
    let bundle = fixture_bundle(card);
    let mut report = well_formed_report(&bundle_commit(&bundle));
    report.claims[1] = ReportClaim {
        id: "ocel.admission.real".into(),
        claim: "OCEL admission rejects flat worlds at the gate.".into(),
        evidence_files: vec!["anti_fake_doctrine.out".into()],
        evidence_snippets: vec!["doctrine_constants_are_used_by_runtime_paths".into()],
        risk_if_false: "World admission would be decorative.".into(),
    };
    match admit(&report, &bundle) {
        Err(ReportAdmissionError::MissingRequiredCitation { required, .. }) => {
            assert_eq!(required, "anti_fake_ocel.out");
        }
        other => panic!("expected MissingRequiredCitation(ocel), got {other:?}"),
    }
}

#[test]
fn kz8_report_diff_detects_dimension_change() {
    let prev = all_true_scorecard();
    let mut curr = all_true_scorecard();
    curr.kz7_runtime_loading_pass = false;
    curr.recompute_overall();
    let d = diff(&prev, &curr);
    assert_eq!(d.changed.len(), 1);
    assert_eq!(d.changed[0].dimension, "kz7_runtime_loading_pass");
    assert!(d.previous_overall);
    assert!(!d.current_overall);
}

#[test]
fn kz8_required_files_list_matches_evidence_loader() {
    // Ensures the prompt + admission citation rules cannot drift away
    // from what the evidence loader requires. If a file moves, both
    // ends must change together.
    for f in &[
        "scorecard.json",
        "git.txt",
        "toolchain.txt",
        "anti_fake_doctrine.out",
        "anti_fake_causal.out",
        "anti_fake_ocel.out",
        "anti_fake_perf.out",
        "anti_fake_packs.out",
        "anti_fake_master.out",
        "ccog.out",
        "autoinstinct.out",
    ] {
        assert!(
            REQUIRED_FILES.contains(f),
            "required-files contract must include {f}"
        );
    }
}

#[test]
fn kz8_no_assert_true_placeholders_remain() {
    // Anti-fake meta-test mirroring anti_fake_packs.rs.
    let src = include_str!("anti_fake_report.rs");
    let needle = format!("{}{}", "assert!(\n        true", ",");
    assert!(
        !src.contains(&needle),
        "release-blocking assert!(true) placeholder must not remain in KZ8 tests"
    );
    let needle2 = format!("{}{}", "assert!(true", ",");
    assert!(
        !src.contains(&needle2),
        "release-blocking assert!(true) placeholder must not remain in KZ8 tests"
    );
}
