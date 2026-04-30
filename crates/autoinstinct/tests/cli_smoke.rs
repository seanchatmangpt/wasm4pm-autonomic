//! `ainst` CLI smoke test using the canonical SPR verb-noun grammar.
//!
//! Drives every PRD §14.1 / SPR §"CLI Grammar" command end-to-end through
//! the actual binary, asserting exit codes, digest propagation, and
//! tamper detection.

#![cfg(feature = "cli")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn ainst_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ainst"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(ainst_bin())
        .args(args)
        .output()
        .expect("ainst run")
}

#[test]
fn cli_full_pipeline_canonical_grammar() {
    let dir = std::env::temp_dir().join("ainst-cli-canonical");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let p = |s: &str| dir.join(s).to_string_lossy().into_owned();

    fs::write(
        p("spec.json"),
        r#"{"name":"smoke","object_types":["https://schema.org/Vehicle"],"event_types":["https://schema.org/Action"],"objects_per_type":2,"events":4}"#,
    )
    .unwrap();

    let r = run(&["generate", "ocel", &p("spec.json"), "--out", &p("log.json")]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));

    let r = run(&["validate", "ocel", &p("log.json")]);
    assert!(r.status.success());

    fs::write(
        p("corpus.json"),
        r#"{"episodes":[
            {"context_urn":"urn:blake3:a","response":"Ask","receipt_urn":"urn:blake3:r1","outcome":"earned"},
            {"context_urn":"urn:blake3:a","response":"Ask","receipt_urn":"urn:blake3:r2","outcome":"earned"},
            {"context_urn":"urn:blake3:b","response":"Inspect","receipt_urn":"urn:blake3:r3","outcome":"earned"}
        ]}"#,
    )
    .unwrap();

    let r = run(&["ingest", "corpus", &p("corpus.json")]);
    assert!(r.status.success());

    let r = run(&[
        "discover",
        "motifs",
        &p("corpus.json"),
        "--min-support",
        "2",
        "--out",
        &p("motifs.json"),
    ]);
    assert!(r.status.success());

    let r = run(&["propose", "policy", &p("motifs.json"), "--out", &p("candidate.json")]);
    assert!(r.status.success());

    let r = run(&["generate", "jtbd", &p("motifs.json"), "--out", &p("scenarios.json")]);
    assert!(r.status.success());

    let r = run(&["run", "gauntlet", &p("candidate.json"), &p("scenarios.json")]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stdout));
    assert!(String::from_utf8_lossy(&r.stdout).contains("ADMITTED"));

    let r = run(&[
        "compile",
        "pack",
        &p("candidate.json"),
        "--name",
        "smoke",
        "--domain",
        "enterprise",
        "--out",
        &p("pack.json"),
    ]);
    assert!(r.status.success());
    assert!(String::from_utf8_lossy(&r.stdout).contains("urn:blake3:"));

    let r = run(&["publish", "pack", &p("pack.json")]);
    assert!(r.status.success());

    let r = run(&[
        "deploy",
        "edge",
        &p("pack.json"),
        "--tier",
        "cloud",
        "--region",
        "us-east-1",
    ]);
    assert!(r.status.success());

    let r = run(&["verify", "replay", &p("pack.manifest.json")]);
    assert!(r.status.success());
    assert!(String::from_utf8_lossy(&r.stdout).contains("OK"));

    let r = run(&["export", "bundle", &p("pack.json"), "--out", &p("bundle.json")]);
    assert!(r.status.success());
    assert!(fs::read(p("bundle.json")).unwrap().len() > 100);
}

#[test]
fn cli_run_gauntlet_rejects_constant_policy() {
    let dir = std::env::temp_dir().join("ainst-cli-reject");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let p = |s: &str| dir.join(s).to_string_lossy().into_owned();

    fs::write(
        p("constant.json"),
        r#"{"rules":[["urn:blake3:a","Ask"],["urn:blake3:b","Ask"]],"default":"Ask"}"#,
    )
    .unwrap();
    fs::write(
        p("scenarios.json"),
        r#"[{"name":"perturb","context_urn":"urn:blake3:a","expected":"Ask","perturbed_context_urn":"urn:blake3:b","forbidden":[]}]"#,
    )
    .unwrap();
    let r = run(&["run", "gauntlet", &p("constant.json"), &p("scenarios.json")]);
    assert!(!r.status.success(), "constant policy must be rejected");
    assert!(String::from_utf8_lossy(&r.stdout).contains("REJECTED"));
}

#[test]
fn cli_verify_replay_detects_tamper() {
    let dir = std::env::temp_dir().join("ainst-cli-tamper");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let p = |s: &str| dir.join(s).to_string_lossy().into_owned();

    fs::write(
        p("candidate.json"),
        r#"{"rules":[["urn:blake3:a","Ask"]],"default":"Ignore"}"#,
    )
    .unwrap();
    let r = run(&[
        "compile",
        "pack",
        &p("candidate.json"),
        "--name",
        "tamper",
        "--domain",
        "enterprise",
        "--out",
        &p("pack.json"),
    ]);
    assert!(r.status.success());
    let r = run(&["publish", "pack", &p("pack.json")]);
    assert!(r.status.success());

    let mut m: serde_json::Value =
        serde_json::from_slice(&fs::read(p("pack.manifest.json")).unwrap()).unwrap();
    m["name"] = serde_json::Value::String("evil".into());
    fs::write(p("pack.manifest.json"), serde_json::to_vec(&m).unwrap()).unwrap();

    let r = run(&["verify", "replay", &p("pack.manifest.json")]);
    assert!(!r.status.success(), "tampered manifest must fail replay");
    assert!(String::from_utf8_lossy(&r.stdout).contains("TAMPER"));
}
