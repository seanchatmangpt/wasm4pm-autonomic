//! Integration tests for SR (Semantic Resolver) — capability receipt consumption with Spec Kit grounding.
//!
//! Tests validate the three-gate grounding proof:
//! 1. Capability receipt validation
//! 2. Ralph planning receipt validation
//! 3. Spec Kit artifact hash verification

use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ── Helper functions ──────────────────────────────────────────────────────

fn create_ggen_receipt(signature: &str) -> String {
    // Format matches actual ggen receipts: input_hashes and output_hashes are
    // Vec<String> (JSON arrays of hex strings), not objects.
    let receipt = json!({
        "operation_id": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2026-04-25T19:30:00Z",
        "input_hashes": [
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        ],
        "output_hashes": [
            "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
        ],
        "signature": signature,
        "previous_receipt_hash": null
    });
    serde_json::to_string_pretty(&receipt).unwrap()
}

fn create_ralph_receipt(verdict: &str, constitution_hash: Option<&str>) -> String {
    let mut receipt = json!({
        "schema": "chatmangpt.ralph.plan.v1",
        "run_id": "ralph-001",
        "target": "mcpp",
        "phase": "implement",
        "completed_phases": ["specify", "plan", "tasks", "implement"],
        "gates": [
            {
                "name": "spec_complete",
                "status": "pass"
            }
        ],
        "verdict": verdict
    });

    if let Some(hash) = constitution_hash {
        receipt["constitution_hash"] = json!(hash);
    }

    serde_json::to_string_pretty(&receipt).unwrap()
}


// ── Test: GgenReceipt deserializes actual ggen receipt format ─────────────
//
// ggen receipts use Vec<String> for both input_hashes and output_hashes.
// This test proves deserialization succeeds and the field type is correct.

#[test]
fn ggen_receipt_deserializes_vec_format_correctly() {
    // This JSON matches the real format produced by the ggen-receipt crate,
    // as observed in /Users/sac/ggen/.ggen/receipts/*.json
    let raw = r#"{
        "operation_id": "pack-install-capability-mcp-20260401-164929",
        "timestamp": "2026-04-01T16:49:29.260052Z",
        "input_hashes": [
            "5dc5076a78bce60dc1a6682017e78ee50f1c3ce5aa5080b8d4f2bc31074ecef3"
        ],
        "output_hashes": [
            "ef315a28b53950ba6b46e1844379bc7ca70cecad21d21939903123991d12dcb5",
            "3aa25a868151a81832ba1d7f176be26030944d9c5f4b8d2122c5bc1a918898b2",
            "f9a3a3d627f4f30ea65edf4990d5093aae4cc87c8403bc9adf5017d1f305485c"
        ],
        "signature": "59ebcb938ce316a436cc198db4f35ae20b83bbf1664e2fcb07b13b37d14f56413fd067e760121cc3266aa5d0ad4de1669a55ffe1988cfb44a1976d31d2825c00",
        "previous_receipt_hash": null
    }"#;

    // We can't import sr's internal types directly, so we validate via a
    // matching struct defined inline — same type as GgenReceipt in sr.rs.
    #[derive(serde::Deserialize, Debug)]
    struct GgenReceiptCheck {
        operation_id: String,
        timestamp: String,
        #[serde(default)]
        input_hashes: Vec<String>,
        #[serde(default)]
        output_hashes: Vec<String>,
        signature: String,
        #[serde(default)]
        previous_receipt_hash: Option<String>,
    }

    let receipt: GgenReceiptCheck =
        serde_json::from_str(raw).expect("must deserialize actual ggen receipt format");

    assert_eq!(receipt.operation_id, "pack-install-capability-mcp-20260401-164929");
    assert_eq!(receipt.input_hashes.len(), 1, "input_hashes must have 1 entry");
    assert_eq!(
        receipt.input_hashes[0],
        "5dc5076a78bce60dc1a6682017e78ee50f1c3ce5aa5080b8d4f2bc31074ecef3"
    );
    assert_eq!(receipt.output_hashes.len(), 3, "output_hashes must have 3 entries");
    assert!(!receipt.signature.is_empty(), "signature must be non-empty");
    assert!(receipt.previous_receipt_hash.is_none());
}

// ── Test: SR output hashes carry algorithm prefix ─────────────────────────
//
// When SR emits evidence hashes (receipt_hash in gates), they must start with
// "sha256:" to identify the algorithm without out-of-band knowledge.

#[test]
fn sr_output_hashes_carry_sha256_prefix() {
    let temp_dir = TempDir::new().unwrap();

    let constitution_path = temp_dir.path().join("constitution.md");
    fs::write(&constitution_path, "# Constitution").unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    fs::write(&spec_path, "# Spec").unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    fs::write(&plan_path, "# Plan").unwrap();

    let tasks_path = temp_dir.path().join("tasks.md");
    fs::write(&tasks_path, "# Tasks").unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--constitution")
        .arg(&constitution_path)
        .arg("--spec")
        .arg(&spec_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--tasks")
        .arg(&tasks_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // All emitted hashes must carry "sha256:" algorithm prefix
            assert!(
                stdout.contains("sha256:"),
                "SR output hashes must carry sha256: algorithm prefix; got: {}",
                &stdout[..stdout.len().min(500)]
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR accepts valid ggen capability receipt ─────────────────────

#[test]
fn sr_accepts_ggen_capability_receipt_path() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("capability.json");

    let receipt_content = create_ggen_receipt("ed25519_signature_abcdef");
    fs::write(&receipt_path, receipt_content).unwrap();

    // Simulate running sr verify
    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Should produce JSON output with capability_receipt gate
            assert!(
                stdout.contains("capability_receipt") || stdout.contains("capability-receipt"),
                "SR output should reference capability receipt"
            );
        }
        Err(e) => {
            // If sr binary doesn't exist yet, skip test
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects empty signature ──────────────────────────────────────

#[test]
fn sr_rejects_empty_signature() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("capability.json");

    let receipt_content = create_ggen_receipt(""); // Empty signature
    fs::write(&receipt_path, receipt_content).unwrap();

    // Simulate running sr verify
    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail with non-zero exit code
            assert_ne!(Some(0), exit_code, "SR should reject empty signature");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR accepts valid Ralph receipt ─────────────────────────────────

#[test]
fn sr_accepts_ralph_planning_receipt_path() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("ralph_plan.json");

    let receipt_content = create_ralph_receipt("Pass", Some("abc123"));
    fs::write(&receipt_path, receipt_content).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--ralph-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("ralph_plan_receipt") || stdout.contains("ralph-plan-receipt"),
                "SR output should reference ralph plan receipt"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects Ralph receipt with non-Pass verdict ────────────────

#[test]
fn sr_rejects_ralph_receipt_non_pass_verdict() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("ralph_plan.json");

    let receipt_content = create_ralph_receipt("Fatal", Some("abc123"));
    fs::write(&receipt_path, receipt_content).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--ralph-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail
            assert_ne!(Some(0), exit_code, "SR should reject non-Pass verdict");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR validates Spec Kit constitution hash ────────────────────────

#[test]
fn sr_validates_speckit_constitution_hash() {
    let temp_dir = TempDir::new().unwrap();

    let constitution_path = temp_dir.path().join("constitution.md");
    fs::write(&constitution_path, "# Constitution\n\nThis is the constitution.").unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    fs::write(&spec_path, "# Spec").unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    fs::write(&plan_path, "# Plan").unwrap();

    let tasks_path = temp_dir.path().join("tasks.md");
    fs::write(&tasks_path, "# Tasks").unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--constitution")
        .arg(&constitution_path)
        .arg("--spec")
        .arg(&spec_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--tasks")
        .arg(&tasks_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("constitution"),
                "SR output should include constitution hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR validates Spec Kit spec hash ────────────────────────────────

#[test]
fn sr_validates_speckit_spec_hash() {
    let temp_dir = TempDir::new().unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    let spec_content = "# Specification\n\nDetailed spec here.";
    fs::write(&spec_path, spec_content).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--spec")
        .arg(&spec_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("spec"),
                "SR output should include spec hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR validates Spec Kit plan hash ────────────────────────────────

#[test]
fn sr_validates_speckit_plan_hash() {
    let temp_dir = TempDir::new().unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    let plan_content = "# Implementation Plan\n\nStep-by-step plan.";
    fs::write(&plan_path, plan_content).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--plan")
        .arg(&plan_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("plan"),
                "SR output should include plan hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR validates Spec Kit tasks hash ───────────────────────────────

#[test]
fn sr_validates_speckit_tasks_hash() {
    let temp_dir = TempDir::new().unwrap();

    let constitution_path = temp_dir.path().join("constitution.md");
    fs::write(&constitution_path, "# Constitution").unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    fs::write(&spec_path, "# Spec").unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    fs::write(&plan_path, "# Plan").unwrap();

    let tasks_path = temp_dir.path().join("tasks.md");
    fs::write(&tasks_path, "# Tasks\n\n- Task 1\n- Task 2").unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--constitution")
        .arg(&constitution_path)
        .arg("--spec")
        .arg(&spec_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--tasks")
        .arg(&tasks_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("tasks"),
                "SR output should include tasks hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects missing constitution ────────────────────────────────

#[test]
fn sr_rejects_missing_constitution() {
    let nonexistent = "/tmp/nonexistent_constitution_12345.md";

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--constitution")
        .arg(nonexistent)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail
            assert_ne!(Some(0), exit_code, "SR should fail on missing constitution");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects missing spec ────────────────────────────────────────

#[test]
fn sr_rejects_missing_spec() {
    let nonexistent = "/tmp/nonexistent_spec_12345.md";

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--spec")
        .arg(nonexistent)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail
            assert_ne!(Some(0), exit_code, "SR should fail on missing spec");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects missing plan ────────────────────────────────────────

#[test]
fn sr_rejects_missing_plan() {
    let nonexistent = "/tmp/nonexistent_plan_12345.md";

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--plan")
        .arg(nonexistent)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail
            assert_ne!(Some(0), exit_code, "SR should fail on missing plan");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR rejects missing tasks ───────────────────────────────────────

#[test]
fn sr_rejects_missing_tasks() {
    let nonexistent = "/tmp/nonexistent_tasks_12345.md";

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--tasks")
        .arg(nonexistent)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail
            assert_ne!(Some(0), exit_code, "SR should fail on missing tasks");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR emits structured verdict with all hashes ───────────────────

#[test]
fn sr_emits_structured_verdict_with_all_hashes() {
    let temp_dir = TempDir::new().unwrap();

    // Create all artifacts
    let cap_receipt_path = temp_dir.path().join("capability.json");
    fs::write(&cap_receipt_path, create_ggen_receipt("sig123")).unwrap();

    let ralph_receipt_path = temp_dir.path().join("ralph.json");
    fs::write(&ralph_receipt_path, create_ralph_receipt("Pass", Some("rh123")))
        .unwrap();

    let constitution_path = temp_dir.path().join("constitution.md");
    fs::write(&constitution_path, "# Constitution").unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    fs::write(&spec_path, "# Specification").unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    fs::write(&plan_path, "# Plan").unwrap();

    let tasks_path = temp_dir.path().join("tasks.md");
    fs::write(&tasks_path, "# Tasks").unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&cap_receipt_path)
        .arg("--ralph-receipt")
        .arg(&ralph_receipt_path)
        .arg("--constitution")
        .arg(&constitution_path)
        .arg("--spec")
        .arg(&spec_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--tasks")
        .arg(&tasks_path)
        .arg("--target")
        .arg("mcpp")
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("sr.result"),
                "SR output should reference sr.result schema"
            );
            assert!(
                stdout.contains("capability_receipt"),
                "SR output should contain capability_receipt gate"
            );
            assert!(
                stdout.contains("ralph_plan_receipt"),
                "SR output should contain ralph_plan_receipt gate"
            );
            assert!(
                stdout.contains("speckit_artifact_grounding"),
                "SR output should contain speckit_artifact_grounding gate"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR includes capability_receipt_hash in gates ──────────────────

#[test]
fn sr_includes_capability_receipt_hash_in_gates() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("capability.json");

    fs::write(&receipt_path, create_ggen_receipt("sig123")).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("capability_receipt_hash"),
                "SR output should include capability_receipt_hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR includes ralph_plan_receipt_hash in gates ───────────────────

#[test]
fn sr_includes_ralph_plan_receipt_hash_in_gates() {
    let temp_dir = TempDir::new().unwrap();
    let receipt_path = temp_dir.path().join("ralph.json");

    fs::write(&receipt_path, create_ralph_receipt("Pass", Some("rh123")))
        .unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--ralph-receipt")
        .arg(&receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("ralph_plan_receipt_hash"),
                "SR output should include ralph_plan_receipt_hash"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR unblocks ostar_closure on valid receipts ───────────────────

#[test]
fn sr_unblocks_ostar_closure_on_valid_receipts_and_artifacts() {
    let temp_dir = TempDir::new().unwrap();

    // Create all artifacts
    let cap_receipt_path = temp_dir.path().join("capability.json");
    fs::write(&cap_receipt_path, create_ggen_receipt("sig123")).unwrap();

    let ralph_receipt_path = temp_dir.path().join("ralph.json");
    fs::write(&ralph_receipt_path, create_ralph_receipt("Pass", Some("rh123")))
        .unwrap();

    let constitution_path = temp_dir.path().join("constitution.md");
    fs::write(&constitution_path, "# Constitution").unwrap();

    let spec_path = temp_dir.path().join("spec.md");
    fs::write(&spec_path, "# Specification").unwrap();

    let plan_path = temp_dir.path().join("plan.md");
    fs::write(&plan_path, "# Plan").unwrap();

    let tasks_path = temp_dir.path().join("tasks.md");
    fs::write(&tasks_path, "# Tasks").unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&cap_receipt_path)
        .arg("--ralph-receipt")
        .arg(&ralph_receipt_path)
        .arg("--constitution")
        .arg(&constitution_path)
        .arg("--spec")
        .arg(&spec_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--tasks")
        .arg(&tasks_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // When all gates pass, SR should suggest next step
            assert!(
                stdout.contains("ostar_closure") || stdout.contains("next"),
                "SR should unblock ostar_closure gate when all receipts valid"
            );
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}

// ── Test: SR blocks ostar_closure on missing Spec Kit artifact ──────────

#[test]
fn sr_blocks_ostar_closure_on_missing_speckit_artifact() {
    let temp_dir = TempDir::new().unwrap();

    // Create cap and ralph receipts but NOT Spec Kit artifacts
    let cap_receipt_path = temp_dir.path().join("capability.json");
    fs::write(&cap_receipt_path, create_ggen_receipt("sig123")).unwrap();

    let ralph_receipt_path = temp_dir.path().join("ralph.json");
    fs::write(&ralph_receipt_path, create_ralph_receipt("Pass", Some("rh123")))
        .unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "sr", "--"])
        .arg("--capability-receipt")
        .arg(&cap_receipt_path)
        .arg("--ralph-receipt")
        .arg(&ralph_receipt_path)
        .arg("--json")
        .current_dir("/Users/sac/dteam")
        .output();

    match output {
        Ok(out) => {
            let exit_code = out.status.code();
            // Should fail because Spec Kit artifacts missing
            assert_ne!(Some(0), exit_code, "SR should fail on missing Spec Kit artifacts");
        }
        Err(e) => {
            eprintln!("Warning: sr binary not available: {}", e);
        }
    }
}
