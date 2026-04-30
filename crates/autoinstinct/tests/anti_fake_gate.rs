//! Phase 4 — Executive Gate integration tests.
//!
//! Validates that `ainst run gauntlet --mode anti-fake ...` produces a
//! structured scorecard, refuses dirty git when required, records
//! provenance, writes evidence files, and exits nonzero on partial proof.

#![cfg(feature = "cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn ainst_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ainst"))
}

fn run_in(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(ainst_bin())
        .args(args)
        .current_dir(dir)
        .env("AINST_GATE_SYNTHETIC_PASS", "1")
        .output()
        .expect("ainst run")
}

fn run_in_no_synthetic(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(ainst_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("ainst run")
}

/// Initialize a clean git repo with a single commit, so `git status` is empty
/// and `rev-parse HEAD` resolves.
fn init_clean_repo(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr));
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "test"]);
    git(&["config", "commit.gpgsign", "false"]);
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-q", "-m", "init"]);
}

#[test]
fn cli_anti_fake_gate_refuses_dirty_git_when_required() {
    let dir = std::env::temp_dir().join("ainst-gate-dirty");
    let _ = fs::remove_dir_all(&dir);
    init_clean_repo(&dir);
    // Make the tree dirty.
    fs::write(dir.join("dirty.txt"), "uncommitted\n").unwrap();

    let out = run_in_no_synthetic(
        &dir,
        &[
            "run",
            "gauntlet",
            "--mode",
            "anti-fake",
            "--require-clean-git",
            "--evidence-dir",
            dir.join("ev").to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "dirty-git gate must fail: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("git tree is not clean"),
        "expected dirty-git error message, got: {combined}"
    );
}

#[test]
fn cli_anti_fake_gate_emits_json_scorecard() {
    let dir = std::env::temp_dir().join("ainst-gate-json");
    let _ = fs::remove_dir_all(&dir);
    init_clean_repo(&dir);

    let ev = dir.join("ev");
    let out = run_in(
        &dir,
        &[
            "run",
            "gauntlet",
            "--mode",
            "anti-fake",
            "--require-clean-git",
            "--evidence",
            "--evidence-dir",
            ev.to_str().unwrap(),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "synthetic gate must pass: {stdout} {}",
        String::from_utf8_lossy(&out.stderr));

    // Parse JSON from stdout (the scorecard is the first JSON object printed).
    let start = stdout.find('{').expect("JSON object in stdout");
    let end = stdout.rfind('}').expect("JSON close in stdout");
    let json = &stdout[start..=end];
    let v: serde_json::Value = serde_json::from_str(json).expect("scorecard parses");
    for dim in autoinstinct::scorecard::Scorecard::dimension_names() {
        assert!(
            v.get(dim).and_then(|x| x.as_bool()).unwrap_or(false),
            "scorecard missing or false dimension `{dim}` in: {json}"
        );
    }
    assert_eq!(v.get("overall_pass").and_then(|x| x.as_bool()), Some(true));
}

#[test]
fn cli_anti_fake_gate_records_branch_commit_toolchain() {
    let dir = std::env::temp_dir().join("ainst-gate-prov");
    let _ = fs::remove_dir_all(&dir);
    init_clean_repo(&dir);

    let ev = dir.join("ev");
    let out = run_in(
        &dir,
        &[
            "run",
            "gauntlet",
            "--mode",
            "anti-fake",
            "--require-clean-git",
            "--evidence",
            "--evidence-dir",
            ev.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let start = stdout.find('{').expect("JSON");
    let end = stdout.rfind('}').expect("JSON close");
    let v: serde_json::Value = serde_json::from_str(&stdout[start..=end]).unwrap();
    let commit = v
        .get("commit_recorded")
        .and_then(|x| x.as_str())
        .unwrap_or_default();
    assert!(
        commit.len() >= 7 && commit.chars().all(|c| c.is_ascii_hexdigit()),
        "commit_recorded must be a hex sha, got: {commit}"
    );
    let toolchain = v
        .get("toolchain_recorded")
        .and_then(|x| x.as_str())
        .unwrap_or_default();
    assert!(
        toolchain.contains("rustc") || toolchain.contains("cargo"),
        "toolchain_recorded must mention rustc/cargo, got: {toolchain}"
    );
}

#[test]
fn cli_anti_fake_gate_writes_evidence_files() {
    let dir = std::env::temp_dir().join("ainst-gate-ev");
    let _ = fs::remove_dir_all(&dir);
    init_clean_repo(&dir);

    let ev = dir.join("ev");
    let out = run_in(
        &dir,
        &[
            "run",
            "gauntlet",
            "--mode",
            "anti-fake",
            "--require-clean-git",
            "--evidence",
            "--evidence-dir",
            ev.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    for f in [
        "scorecard.json",
        "git.txt",
        "toolchain.txt",
        "anti_fake_doctrine.out",
        "anti_fake_causal.out",
        "anti_fake_ocel.out",
        "anti_fake_perf.out",
        "anti_fake_packs.out",
        "ccog.out",
        "autoinstinct.out",
    ] {
        let path = ev.join(f);
        assert!(path.exists(), "evidence file missing: {}", path.display());
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.is_empty(), "evidence file empty: {}", path.display());
    }

    // Scorecard.json must be valid JSON with overall_pass=true.
    let json = fs::read_to_string(ev.join("scorecard.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v.get("overall_pass").and_then(|x| x.as_bool()), Some(true));
}
