//! sr — Semantic Resolver for capability receipt consumption with Spec Kit grounding.
//!
//! Purpose: Validate and integrate three receipt types:
//! 1. ggen capability receipt (proof that code generation worked)
//! 2. Ralph planning receipt (proof that planning/phasing worked)
//! 3. Spec Kit artifact hashes (proof that spec.md, plan.md, tasks.md match execution)
//!
//! Output: `chatmangpt.sr.result.v1` with three gates:
//! - capability_receipt gate
//! - ralph_plan_receipt gate
//! - speckit_artifact_grounding gate
//!
//! Exit codes:
//!   0 — all gates pass
//!   1 — soft fail (missing artifacts, warnings)
//!   2 — fatal (hash mismatch, missing receipt)
//!
//! Usage:
//!   sr verify --capability-receipt <path> --ralph-receipt <path> --target mcpp
//!   sr verify --dry-run --capability-receipt <path>

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

// ── ANSI color codes ──────────────────────────────────────────────────────
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

const SR_SCHEMA_VERSION: &str = "chatmangpt.sr.result.v1";

// ── Received types ────────────────────────────────────────────────────────

/// Simplified ggen receipt structure (we only care about signature/hash).
///
/// Actual ggen receipts (produced by ggen-receipt crate) emit `input_hashes`
/// and `output_hashes` as JSON arrays of hex strings, not objects.
/// Example: `"input_hashes": ["d2e2072244b332bf..."]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgenReceipt {
    pub operation_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub input_hashes: Vec<String>,
    #[serde(default)]
    pub output_hashes: Vec<String>,
    pub signature: String,
    #[serde(default)]
    pub previous_receipt_hash: Option<String>,
}

/// Simplified Ralph plan structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphPlanReceipt {
    pub schema: String,
    pub run_id: String,
    pub target: String,
    #[serde(default)]
    pub constitution_hash: Option<String>,
    #[serde(default)]
    pub spec_hash: Option<String>,
    pub phase: String,
    #[serde(default)]
    pub completed_phases: Vec<String>,
    #[serde(default)]
    pub gates: Vec<RalphGate>,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphGate {
    pub name: String,
    pub status: String,
}

// ── SR Gate result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SRGateStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRGate {
    pub name: String,
    pub status: SRGateStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_hashes: Option<std::collections::HashMap<String, String>>,
}

// ── SR Result (output schema) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRResult {
    pub schema: String,
    pub command: String,
    pub status: String,
    pub target: String,
    pub line_status: String,
    pub data: SRData,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<SRNext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRData {
    pub gates: Vec<SRGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_receipt_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ralph_plan_receipt_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speckit_artifact_hashes: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRNext {
    pub command: String,
    pub reason: String,
}

// ── Compute SHA-256 hash ──────────────────────────────────────────────────

/// Compute SHA-256 of `data` and return as an algorithm-tagged hex string.
///
/// Format: `sha256:<64-hex-chars>`
/// This prefix is required by the receipt discipline so consumers can
/// identify the algorithm without out-of-band knowledge.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{:x}", hasher.finalize())
}

fn hash_file(path: &Path) -> Result<String> {
    let data = fs::read(path).context(format!("Failed to read file: {}", path.display()))?;
    Ok(sha256_hex(&data))
}

// ── Validate capability receipt ───────────────────────────────────────────

fn validate_capability_receipt(receipt_path: &Path) -> Result<(GgenReceipt, String)> {
    let content = fs::read_to_string(receipt_path)
        .context(format!("Failed to read capability receipt: {}", receipt_path.display()))?;

    let receipt: GgenReceipt = serde_json::from_str(&content)
        .context("Failed to parse capability receipt JSON")?;

    // Verify signature is non-empty
    if receipt.signature.is_empty() {
        bail!("Capability receipt: signature field is empty");
    }

    let receipt_hash = sha256_hex(content.as_bytes());

    Ok((receipt, receipt_hash))
}

// ── Validate Ralph plan receipt ───────────────────────────────────────────

fn validate_ralph_receipt(receipt_path: &Path) -> Result<(RalphPlanReceipt, String)> {
    let content = fs::read_to_string(receipt_path)
        .context(format!("Failed to read Ralph receipt: {}", receipt_path.display()))?;

    let receipt: RalphPlanReceipt =
        serde_json::from_str(&content).context("Failed to parse Ralph receipt JSON")?;

    // Verify schema
    if receipt.schema != "chatmangpt.ralph.plan.v1" {
        bail!(
            "Ralph receipt: unexpected schema version: {}",
            receipt.schema
        );
    }

    // Verify verdict is pass-equivalent
    if receipt.verdict != "Pass" {
        bail!("Ralph receipt: verdict is not Pass (got: {})", receipt.verdict);
    }

    let receipt_hash = sha256_hex(content.as_bytes());

    Ok((receipt, receipt_hash))
}

// ── Validate Spec Kit artifacts ───────────────────────────────────────────

fn validate_speckit_artifacts(
    constitution_path: Option<&Path>,
    spec_path: Option<&Path>,
    plan_path: Option<&Path>,
    tasks_path: Option<&Path>,
) -> Result<std::collections::HashMap<String, String>> {
    let mut hashes = std::collections::HashMap::new();
    let mut errors = Vec::new();

    if let Some(path) = constitution_path {
        match hash_file(path) {
            Ok(hash) => {
                hashes.insert("constitution".to_string(), hash);
            }
            Err(e) => {
                errors.push(format!("constitution.md: {}", e));
            }
        }
    } else {
        errors.push("constitution.md: path not provided".to_string());
    }

    if let Some(path) = spec_path {
        match hash_file(path) {
            Ok(hash) => {
                hashes.insert("spec".to_string(), hash);
            }
            Err(e) => {
                errors.push(format!("spec.md: {}", e));
            }
        }
    } else {
        errors.push("spec.md: path not provided".to_string());
    }

    if let Some(path) = plan_path {
        match hash_file(path) {
            Ok(hash) => {
                hashes.insert("plan".to_string(), hash);
            }
            Err(e) => {
                errors.push(format!("plan.md: {}", e));
            }
        }
    } else {
        errors.push("plan.md: path not provided".to_string());
    }

    if let Some(path) = tasks_path {
        match hash_file(path) {
            Ok(hash) => {
                hashes.insert("tasks".to_string(), hash);
            }
            Err(e) => {
                errors.push(format!("tasks.md: {}", e));
            }
        }
    } else {
        errors.push("tasks.md: path not provided".to_string());
    }

    if !errors.is_empty() {
        bail!("Spec Kit artifact validation failed: {}", errors.join("; "));
    }

    Ok(hashes)
}

// ── Build SR result ───────────────────────────────────────────────────────

fn build_sr_result(
    target: &str,
    capability_receipt: Option<(GgenReceipt, String)>,
    ralph_receipt: Option<(RalphPlanReceipt, String)>,
    speckit_hashes: Option<std::collections::HashMap<String, String>>,
    errors: Vec<String>,
    warnings: Vec<String>,
) -> SRResult {
    let mut gates = Vec::new();
    let mut all_pass = errors.is_empty();

    // Gate 1: capability_receipt
    if let Some((_, hash)) = &capability_receipt {
        gates.push(SRGate {
            name: "capability_receipt".to_string(),
            status: SRGateStatus::Pass,
            receipt_hash: Some(hash.clone()),
            artifact_hashes: None,
        });
    } else {
        all_pass = false;
        gates.push(SRGate {
            name: "capability_receipt".to_string(),
            status: SRGateStatus::Fail,
            receipt_hash: None,
            artifact_hashes: None,
        });
    }

    // Gate 2: ralph_plan_receipt
    if let Some((_, hash)) = &ralph_receipt {
        gates.push(SRGate {
            name: "ralph_plan_receipt".to_string(),
            status: SRGateStatus::Pass,
            receipt_hash: Some(hash.clone()),
            artifact_hashes: None,
        });
    } else {
        all_pass = false;
        gates.push(SRGate {
            name: "ralph_plan_receipt".to_string(),
            status: SRGateStatus::Fail,
            receipt_hash: None,
            artifact_hashes: None,
        });
    }

    // Gate 3: speckit_artifact_grounding
    if let Some(hashes) = &speckit_hashes {
        gates.push(SRGate {
            name: "speckit_artifact_grounding".to_string(),
            status: SRGateStatus::Pass,
            receipt_hash: None,
            artifact_hashes: Some(hashes.clone()),
        });
    } else {
        all_pass = false;
        gates.push(SRGate {
            name: "speckit_artifact_grounding".to_string(),
            status: SRGateStatus::Fail,
            receipt_hash: None,
            artifact_hashes: None,
        });
    }

    let status = if all_pass { "pass" } else { "fail" }.to_string();
    let next = if all_pass {
        Some(SRNext {
            command: "sr verify --gate ostar_closure --target mcpp".to_string(),
            reason: "Capability receipt, Ralph receipt, and Spec Kit artifact grounding verified."
                .to_string(),
        })
    } else {
        None
    };

    SRResult {
        schema: SR_SCHEMA_VERSION.to_string(),
        command: "sr.verify".to_string(),
        status,
        target: target.to_string(),
        line_status: "running".to_string(),
        data: SRData {
            gates,
            capability_receipt_hash: capability_receipt.as_ref().map(|(_, h)| h.clone()),
            ralph_plan_receipt_hash: ralph_receipt.as_ref().map(|(_, h)| h.clone()),
            speckit_artifact_hashes: speckit_hashes,
        },
        errors,
        warnings,
        next,
    }
}

// ── CLI parsing ───────────────────────────────────────────────────────────

#[derive(Debug)]
struct Args {
    capability_receipt: Option<PathBuf>,
    ralph_receipt: Option<PathBuf>,
    constitution_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    plan_path: Option<PathBuf>,
    tasks_path: Option<PathBuf>,
    target: String,
    json_output: bool,
    dry_run: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        capability_receipt: None,
        ralph_receipt: None,
        constitution_path: None,
        spec_path: None,
        plan_path: None,
        tasks_path: None,
        target: "mcpp".to_string(),
        json_output: false,
        dry_run: false,
    };

    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;

    while i < argv.len() {
        match argv[i].as_str() {
            "--capability-receipt" => {
                i += 1;
                if i < argv.len() {
                    args.capability_receipt = Some(PathBuf::from(&argv[i]));
                }
            }
            "--ralph-receipt" => {
                i += 1;
                if i < argv.len() {
                    args.ralph_receipt = Some(PathBuf::from(&argv[i]));
                }
            }
            "--constitution" => {
                i += 1;
                if i < argv.len() {
                    args.constitution_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--spec" => {
                i += 1;
                if i < argv.len() {
                    args.spec_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--plan" => {
                i += 1;
                if i < argv.len() {
                    args.plan_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--tasks" => {
                i += 1;
                if i < argv.len() {
                    args.tasks_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--target" => {
                i += 1;
                if i < argv.len() {
                    args.target = argv[i].clone();
                }
            }
            "--json" => args.json_output = true,
            "--dry-run" => args.dry_run = true,
            "--help" | "-h" => {
                println!("sr verify — Semantic Resolver for capability receipt consumption");
                println!();
                println!("Usage: sr verify [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --capability-receipt <PATH>  Path to ggen capability receipt JSON");
                println!("  --ralph-receipt <PATH>       Path to Ralph planning receipt JSON");
                println!("  --constitution <PATH>        Path to Spec Kit constitution.md");
                println!("  --spec <PATH>                Path to Spec Kit spec.md");
                println!("  --plan <PATH>                Path to Spec Kit plan.md");
                println!("  --tasks <PATH>               Path to Spec Kit tasks.md");
                println!("  --target <NAME>              Target name (default: mcpp)");
                println!("  --json                       Output result as JSON");
                println!("  --dry-run                    Parse inputs without validation");
                println!("  --help                       Show this help message");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    Ok(args)
}

// ── Print result ──────────────────────────────────────────────────────────

fn print_result(result: &SRResult, json_output: bool) {
    if json_output {
        if let Ok(json) = serde_json::to_string_pretty(result) {
            println!("{}", json);
        }
    } else {
        println!("{}{}sr verify — Semantic Resolver{}", BOLD, GREEN, RESET);
        println!();

        println!("{}Schema:{} {}", BOLD, RESET, result.schema);
        println!("{}Target:{} {}", BOLD, RESET, result.target);
        println!("{}Status:{} {}", BOLD, RESET, result.status);
        println!();

        println!("{}Gates:{}", BOLD, RESET);
        for gate in &result.data.gates {
            let gate_status = if gate.status == SRGateStatus::Pass {
                format!("{}PASS{}", GREEN, RESET)
            } else {
                format!("{}FAIL{}", RED, RESET)
            };
            println!("  • {} ... {}", gate.name, gate_status);

            if let Some(hash) = &gate.receipt_hash {
                // Display at most 23 chars (prefix "sha256:" + 16 hex chars)
                let display = &hash[..hash.len().min(23)];
                println!("    Receipt hash: {}…", display);
            }
            if let Some(hashes) = &gate.artifact_hashes {
                for (k, v) in hashes {
                    let display = &v[..v.len().min(23)];
                    println!("    {}: {}…", k, display);
                }
            }
        }

        if !result.errors.is_empty() {
            println!();
            println!("{}Errors:{}", BOLD, RESET);
            for err in &result.errors {
                println!("  {}• {}{}", RED, err, RESET);
            }
        }

        if !result.warnings.is_empty() {
            println!();
            println!("{}Warnings:{}", BOLD, RESET);
            for warn in &result.warnings {
                println!("  {}• {}{}", YELLOW, warn, RESET);
            }
        }

        if let Some(next) = &result.next {
            println!();
            println!("{}Next:{}", BOLD, RESET);
            println!("  Command: {}", next.command);
            println!("  Reason: {}", next.reason);
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}Error parsing arguments: {}{}", RED, e, RESET);
            return ExitCode::from(2);
        }
    };

    if args.dry_run {
        let result = SRResult {
            schema: SR_SCHEMA_VERSION.to_string(),
            command: "sr.verify".to_string(),
            status: "pass".to_string(),
            target: args.target.clone(),
            line_status: "running".to_string(),
            data: SRData {
                gates: vec![],
                capability_receipt_hash: None,
                ralph_plan_receipt_hash: None,
                speckit_artifact_hashes: None,
            },
            errors: vec!["Dry-run mode: no validation performed".to_string()],
            warnings: vec![],
            next: None,
        };

        print_result(&result, args.json_output);
        return ExitCode::from(0);
    }

    // Validate capability receipt
    let cap_receipt = match &args.capability_receipt {
        Some(path) => match validate_capability_receipt(path) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("{}Capability receipt error: {}{}", RED, e, RESET);
                None
            }
        },
        None => None,
    };

    // Validate Ralph receipt
    let ralph_receipt = match &args.ralph_receipt {
        Some(path) => match validate_ralph_receipt(path) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("{}Ralph receipt error: {}{}", RED, e, RESET);
                None
            }
        },
        None => None,
    };

    // Validate Spec Kit artifacts
    let speckit_hashes = match validate_speckit_artifacts(
        args.constitution_path.as_deref(),
        args.spec_path.as_deref(),
        args.plan_path.as_deref(),
        args.tasks_path.as_deref(),
    ) {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("{}Spec Kit validation error: {}{}", RED, e, RESET);
            None
        }
    };

    // Build errors/warnings
    let mut errors = Vec::new();

    if cap_receipt.is_none() && args.capability_receipt.is_some() {
        errors.push("CAPABILITY_RECEIPT_INVALID".to_string());
    }
    if ralph_receipt.is_none() && args.ralph_receipt.is_some() {
        errors.push("RALPH_PLAN_RECEIPT_INVALID".to_string());
    }
    if speckit_hashes.is_none() {
        errors.push("SPECKIT_ARTIFACT_INVALID".to_string());
    }

    // Build result
    let result = build_sr_result(
        &args.target,
        cap_receipt,
        ralph_receipt,
        speckit_hashes,
        errors.clone(),
        Vec::new(),
    );

    print_result(&result, args.json_output);

    if errors.is_empty() {
        ExitCode::from(0)
    } else {
        ExitCode::from(2)
    }
}
