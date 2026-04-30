//! KZ8 — CLI integration tests for `ainst report validate` and
//! `ainst report diff`. Live `ainst report generate` (which shells out
//! to Gemini) is exercised manually.

#![cfg(feature = "cli")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use autoinstinct::report::schema::{
    GeneratedReport, OpenRisk, ReportClaim, ReportKind, ReportStatus, RiskSeverity,
};
use autoinstinct::scorecard::{all_true_scorecard, Scorecard};

fn ainst_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ainst"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(ainst_bin())
        .args(args)
        .output()
        .expect("ainst run")
}

fn write_evidence_dir(dir: &Path, scorecard: &Scorecard) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("scorecard.json"), scorecard.to_json().unwrap()).unwrap();
    fs::write(dir.join("git.txt"), "branch: x\ncommit: abc\nclean: true\n").unwrap();
    fs::write(dir.join("toolchain.txt"), "rustc 1.0.0\n").unwrap();
    let mut entries: BTreeMap<&str, &str> = BTreeMap::new();
    entries.insert(
        "anti_fake_doctrine.out",
        "test doctrine_constants_are_used_by_runtime_paths ... ok\n",
    );
    entries.insert(
        "anti_fake_causal.out",
        "test causal_every_perturbation_changes_response ... ok\n",
    );
    entries.insert(
        "anti_fake_ocel.out",
        "test ocel_admitted_world_produces_nonempty_corpus ... ok\n",
    );
    entries.insert(
        "anti_fake_perf.out",
        "test anti_fake_decide_is_zero_heap_and_input_dependent ... ok\n",
    );
    entries.insert(
        "anti_fake_packs.out",
        "test kz7b_pack_activation_changes_decision_surface ... ok\nmatched_rule_id observed.\n",
    );
    entries.insert(
        "anti_fake_master.out",
        "test master_ocel_to_pack_to_ccog_runtime_to_proof ... ok\n",
    );
    entries.insert("ccog.out", "test result: ok. 196 passed\n");
    entries.insert("autoinstinct.out", "test result: ok. 118 passed\n");
    for (name, body) in entries {
        fs::write(dir.join(name), body).unwrap();
    }
}

fn well_formed_report(commit: &str) -> GeneratedReport {
    GeneratedReport {
        report_kind: ReportKind::Executive,
        title: "Anti-Fake Substrate Closure".into(),
        commit: commit.into(),
        toolchain: "rustc 1.0.0".into(),
        overall_status: ReportStatus::Pass,
        claims: vec![
            ReportClaim {
                id: "kz7.runtime.pack.observable".into(),
                claim: "Loaded pack changes runtime decision and exposes matched_rule_id.".into(),
                evidence_files: vec!["anti_fake_packs.out".into()],
                evidence_snippets: vec![
                    "kz7b_pack_activation_changes_decision_surface".into(),
                    "matched_rule_id".into(),
                ],
                risk_if_false: "Pack layer would be decorative.".into(),
            },
            ReportClaim {
                id: "master.loop.proven".into(),
                claim: "The master loop from world generation to ccog runtime is proven.".into(),
                evidence_files: vec!["anti_fake_master.out".into()],
                evidence_snippets: vec!["master_ocel_to_pack_to_ccog_runtime_to_proof".into()],
                risk_if_false: "Composition between subsystems would be unproven.".into(),
            },
        ],
        open_risks: vec![OpenRisk {
            id: "ent.ops.hardening".into(),
            risk: "Enterprise operational hardening remains.".into(),
            severity: RiskSeverity::P1,
            mitigation: "Track signed pack distribution as follow-up.".into(),
        }],
        markdown: "# Anti-Fake Substrate Closure\n\nThe master loop is proven.\n".into(),
    }
}

#[test]
fn kz8_cli_report_validate_admits_well_formed_fixture() {
    let dir = std::env::temp_dir().join("ainst-kz8-validate-ok");
    let _ = fs::remove_dir_all(&dir);
    let ev = dir.join("ev");
    let mut card = all_true_scorecard();
    card.commit_recorded = "f497664".into();
    write_evidence_dir(&ev, &card);
    let report = well_formed_report(&card.commit_recorded);
    let report_path = dir.join("report.json");
    fs::create_dir_all(&dir).unwrap();
    fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();

    let out = run(&[
        "report",
        "validate",
        report_path.to_str().unwrap(),
        "--evidence-dir",
        ev.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("ADMITTED"));
}

#[test]
fn kz8_cli_report_validate_rejects_overclaim_fixture() {
    let dir = std::env::temp_dir().join("ainst-kz8-validate-overclaim");
    let _ = fs::remove_dir_all(&dir);
    let ev = dir.join("ev");
    let mut card = all_true_scorecard();
    card.commit_recorded = "f497664".into();
    write_evidence_dir(&ev, &card);
    let mut report = well_formed_report(&card.commit_recorded);
    report.markdown.push_str("\nThe system is SOC2 certified.\n");
    let report_path = dir.join("report.json");
    fs::create_dir_all(&dir).unwrap();
    fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();

    let out = run(&[
        "report",
        "validate",
        report_path.to_str().unwrap(),
        "--evidence-dir",
        ev.to_str().unwrap(),
    ]);
    assert!(!out.status.success(), "overclaim must reject");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("REJECTED") || stdout.contains("forbidden over-claim"),
        "expected rejection message, got: {stdout}"
    );
}

#[test]
fn kz8_cli_report_diff_emits_dimension_change() {
    let dir = std::env::temp_dir().join("ainst-kz8-diff");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let prev = all_true_scorecard();
    let mut curr = all_true_scorecard();
    curr.kz7_runtime_loading_pass = false;
    curr.recompute_overall();
    let prev_path = dir.join("prev.json");
    let curr_path = dir.join("curr.json");
    fs::write(&prev_path, prev.to_json().unwrap()).unwrap();
    fs::write(&curr_path, curr.to_json().unwrap()).unwrap();

    let out = run(&[
        "report",
        "diff",
        "--previous",
        prev_path.to_str().unwrap(),
        "--current",
        curr_path.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("kz7_runtime_loading_pass"),
        "diff must mention changed dimension, got: {stdout}"
    );
    assert!(
        stdout.contains("true -> false") || stdout.contains("\"current\": false"),
        "diff must show direction, got: {stdout}"
    );
}
