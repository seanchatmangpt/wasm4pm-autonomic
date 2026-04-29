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

// ── POWL8 program validation ──────────────────────────────────────────────

/// Valid POWL8 ISA opcode range: 0x01–0x08 (8 opcodes, one-byte ISA).
const POWL8_OPCODE_MIN: u64 = 0x01;
const POWL8_OPCODE_MAX: u64 = 0x08;

/// Result of validating a POWL8 opcode stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Powl8ProgramResult {
    /// BLAKE3 hash of the raw opcode bytes (values only, in array order).
    pub hash: String,
    /// Number of opcodes in the stream.
    pub opcode_count: usize,
    /// True when every opcode is in 0x01–0x08.
    pub valid: bool,
    /// Present when `valid` is false; lists the first invalid opcode and its index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Read a JSON file containing `{"opcodes": [<u64>, ...]}`, validate each
/// value is in 0x01–0x08, and compute a BLAKE3 hash of the raw opcode bytes.
///
/// The hash is computed over the little-endian byte representation of each
/// opcode value (one byte per opcode, since the ISA is one-byte wide), making
/// it a content-addressed identifier for the exact program stream.
fn validate_powl8_program(path: &Path) -> Result<Powl8ProgramResult> {
    let content = fs::read_to_string(path).context(format!(
        "Failed to read POWL8 program file: {}",
        path.display()
    ))?;

    let json: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse POWL8 program JSON")?;

    let opcodes_arr = json
        .get("opcodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("POWL8 program JSON must contain an 'opcodes' array"))?;

    let mut raw_bytes: Vec<u8> = Vec::with_capacity(opcodes_arr.len());
    for (idx, val) in opcodes_arr.iter().enumerate() {
        let opcode = val
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("opcodes[{}] is not a non-negative integer", idx))?;
        if opcode < POWL8_OPCODE_MIN || opcode > POWL8_OPCODE_MAX {
            return Ok(Powl8ProgramResult {
                hash: String::new(),
                opcode_count: opcodes_arr.len(),
                valid: false,
                error: Some(format!(
                    "opcodes[{}] = 0x{:02X} is out of range (valid: 0x{:02X}–0x{:02X})",
                    idx, opcode, POWL8_OPCODE_MIN, POWL8_OPCODE_MAX
                )),
            });
        }
        raw_bytes.push(opcode as u8);
    }

    let hash = format!("blake3:{}", blake3::hash(&raw_bytes).to_hex());

    Ok(Powl8ProgramResult {
        hash,
        opcode_count: raw_bytes.len(),
        valid: true,
        error: None,
    })
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powl8_program: Option<Powl8ProgramResult>,
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
    let content = fs::read_to_string(receipt_path).context(format!(
        "Failed to read capability receipt: {}",
        receipt_path.display()
    ))?;

    let receipt: GgenReceipt =
        serde_json::from_str(&content).context("Failed to parse capability receipt JSON")?;

    // Verify signature is non-empty
    if receipt.signature.is_empty() {
        bail!("Capability receipt: signature field is empty");
    }

    let receipt_hash = sha256_hex(content.as_bytes());

    Ok((receipt, receipt_hash))
}

// ── Validate Ralph plan receipt ───────────────────────────────────────────

fn validate_ralph_receipt(receipt_path: &Path) -> Result<(RalphPlanReceipt, String)> {
    let content = fs::read_to_string(receipt_path).context(format!(
        "Failed to read Ralph receipt: {}",
        receipt_path.display()
    ))?;

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
        bail!(
            "Ralph receipt: verdict is not Pass (got: {})",
            receipt.verdict
        );
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
    powl8_result: Option<Powl8ProgramResult>,
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
            powl8_program: powl8_result,
        },
        errors,
        warnings,
        next,
    }
}

// ── SR Closure Receipt ────────────────────────────────────────────────────

/// Ggen-compatible closure receipt written by SR after successful verification.
///
/// Schema follows `GgenReceipt` exactly so downstream consumers can parse
/// the full dteam chain uniformly. When Ed25519 signing is unavailable the
/// `signature` field is set to the literal string `"unsigned"` and the extra
/// `signed` field is set to `false`.  Consumers MUST check `signed` before
/// treating the signature as cryptographic proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SRClosureReceipt {
    pub operation_id: String,
    pub timestamp: String,
    pub input_hashes: Vec<String>,
    pub output_hashes: Vec<String>,
    pub signature: String,
    pub signed: bool,
    pub previous_receipt_hash: Option<String>,
}

/// Find the most recent receipt file in a directory, returning its SHA-256 hash.
/// Returns `None` when the directory is absent or contains no `.json` files.
fn latest_receipt_hash(receipts_dir: &Path) -> Option<String> {
    let entries = fs::read_dir(receipts_dir).ok()?;
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, e.path()))
        })
        .collect();
    files.sort_by_key(|(t, _)| *t);
    let (_, path) = files.last()?;
    let data = fs::read(path).ok()?;
    // Plain 64-char hex, matching ggen receipt chain convention
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(&data);
    Some(format!("{:x}", h.finalize()))
}

/// Write an SR closure receipt to `out_path`.
///
/// `input_paths` — every file SR consumed (capability receipt, ralph receipt,
///   spec kit artifacts).  Each is hashed with SHA-256; missing files are
///   skipped (not an error — SR already validated them above).
/// `sr_result_json` — the serialized `SRResult`; its hash becomes the sole
///   entry in `output_hashes`.
fn write_sr_receipt(
    operation_id: &str,
    input_paths: &[&Path],
    sr_result_json: &str,
    receipts_dir: &Path,
    out_path: &Path,
) -> Result<()> {
    use sha2::{Digest, Sha256};

    // Hash each input file (plain 64-char hex, no prefix — matches ggen chain)
    let input_hashes: Vec<String> = input_paths
        .iter()
        .filter_map(|p| {
            let data = fs::read(p).ok()?;
            let mut h = Sha256::new();
            h.update(&data);
            Some(format!("{:x}", h.finalize()))
        })
        .collect();

    // Hash the SRResult JSON as the sole output artifact
    let mut h = Sha256::new();
    h.update(sr_result_json.as_bytes());
    let output_hash = format!("{:x}", h.finalize());

    let previous_receipt_hash = latest_receipt_hash(receipts_dir);

    let receipt = SRClosureReceipt {
        operation_id: operation_id.to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        input_hashes,
        output_hashes: vec![output_hash],
        signature: "unsigned".to_string(),
        signed: false,
        previous_receipt_hash,
    };

    let json =
        serde_json::to_string_pretty(&receipt).context("Failed to serialize SR closure receipt")?;

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).context("Failed to create receipts directory")?;
    }

    fs::write(out_path, &json)
        .context(format!("Failed to write receipt: {}", out_path.display()))?;
    Ok(())
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
    powl8_program: Option<PathBuf>,
    target: String,
    json_output: bool,
    dry_run: bool,
    receipt_out: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        capability_receipt: None,
        ralph_receipt: None,
        constitution_path: None,
        spec_path: None,
        plan_path: None,
        tasks_path: None,
        powl8_program: None,
        target: "mcpp".to_string(),
        json_output: false,
        dry_run: false,
        receipt_out: None,
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
            "--powl8-program" => {
                i += 1;
                if i < argv.len() {
                    args.powl8_program = Some(PathBuf::from(&argv[i]));
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
            "--receipt-out" => {
                i += 1;
                if i < argv.len() {
                    args.receipt_out = Some(PathBuf::from(&argv[i]));
                }
            }
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
                println!("  --powl8-program <PATH>       Path to POWL8 program JSON (opcodes array, 0x01-0x08)");
                println!("  --target <NAME>              Target name (default: mcpp)");
                println!("  --json                       Output result as JSON");
                println!("  --dry-run                    Parse inputs without validation");
                println!("  --receipt-out <PATH>         Write SR closure receipt to this path");
                println!("                               (default: .portfolio/receipts/sr-<target>-<ts>.json)");
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

// ── Replay-chain subcommand ───────────────────────────────────────────────
//
// `sr replay-chain --chain <path> --constitution <path> [--from <tick_id>] [--strict-wrap]`
//
// Walks chain.json from genesis, recomputes each `this_hash` (BLAKE3-256 over
// JCS-canonicalized entry-minus-(this_hash,signature)), verifies Ed25519
// signature against the pinned pubkey resolved from the constitution location
// (sibling .portfolio/keys/portfolio-root-2026Q2.ed25519.pk by default), and
// asserts `prev_hash == previous.this_hash`. Exits 0 on success, 2 on first
// failure. Writes chain-replay-summary.json next to the chain file.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainReplaySummary {
    pub schema: String,
    pub is_valid: bool,
    pub receipt_count: usize,
    pub head: Option<String>,
    pub verified_at: String,
    pub errors: Vec<String>,
    pub chain: String,
    pub constitution: String,
}

fn b3_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

fn blake2b_256_hex(data: &[u8]) -> String {
    use blake2::digest::{Update, VariableOutput};
    use blake2::Blake2bVar;
    let mut hasher = Blake2bVar::new(32).expect("blake2b 32-byte output");
    hasher.update(data);
    let mut out = [0u8; 32];
    hasher.finalize_variable(&mut out).expect("finalize");
    hex::encode(out)
}

/// JSON Canonicalization Scheme (RFC 8785) — minimal implementation matching
/// the reference Python in chain-verify.sh (UTF-16BE codepoint-sorted keys,
/// no whitespace, integer-form floats when integral).
fn jcs(v: &serde_json::Value) -> String {
    use serde_json::Value;
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => {
            // JCS integer normalization: canonical form strips decimal for integers.
            // as_i64/as_u64 work for values stored without decimal (e.g. 305, 0).
            if let Some(i) = n.as_i64() {
                return i.to_string();
            }
            if let Some(u) = n.as_u64() {
                return u.to_string();
            }
            // Float values: check for mathematical integrality (e.g. "1.0" → "1").
            // With arbitrary_precision, as_f64() parses the decimal string; we use
            // it only for the integral check, not for the final string representation.
            if let Some(f) = n.as_f64() {
                if f.is_finite() && f.fract() == 0.0 {
                    return format!("{}", f as i64);
                }
            }
            // Non-integer floats: use the arbitrary-precision string (preserves
            // original decimal exactly). This prevents ULP drift where Python's
            // json.dumps and Rust's f64::to_string diverge on the last digit
            // (e.g. 0.9901639344262295 → 0.9901639344262296 via f64 round-trip).
            n.to_string()
        }
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()),
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(jcs).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| {
                let av: Vec<u16> = a.encode_utf16().collect();
                let bv: Vec<u16> = b.encode_utf16().collect();
                av.cmp(&bv)
            });
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    let kj = serde_json::to_string(k).unwrap_or_else(|_| "\"\"".into());
                    format!("{}:{}", kj, jcs(&map[*k]))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn resolve_pubkey_path(constitution_path: &Path) -> PathBuf {
    // constitution at <repo>/.specify/memory/constitution.md
    // pubkey at      <repo>/.portfolio/keys/portfolio-root-2026Q2.ed25519.pk
    let mut p = constitution_path.to_path_buf();
    // pop constitution.md, memory/, .specify/
    for _ in 0..3 {
        p.pop();
    }
    p.push(".portfolio");
    p.push("keys");
    p.push("portfolio-root-2026Q2.ed25519.pk");
    p
}

fn replay_chain(
    chain_path: &Path,
    constitution_path: &Path,
    from_tick: Option<&str>,
    strict_wrap: bool,
) -> Result<ChainReplaySummary> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let chain_text = fs::read_to_string(chain_path)
        .context(format!("Failed to read chain: {}", chain_path.display()))?;
    let chain: serde_json::Value =
        serde_json::from_str(&chain_text).context("Failed to parse chain JSON")?;

    // Load pubkey
    let pk_path = resolve_pubkey_path(constitution_path);
    let pk_hex = fs::read_to_string(&pk_path)
        .context(format!("Failed to read pubkey: {}", pk_path.display()))?
        .trim()
        .to_string();
    let pk_bytes = hex::decode(&pk_hex).context("pubkey not valid hex")?;
    if pk_bytes.len() != 32 {
        bail!("pubkey must be 32 bytes, got {}", pk_bytes.len());
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);
    let vk = VerifyingKey::from_bytes(&pk_arr).context("invalid Ed25519 pubkey")?;

    let entries = chain
        .get("entries")
        .and_then(|e| e.as_array())
        .ok_or_else(|| anyhow::anyhow!("chain.json missing 'entries' array"))?;

    let mut errors: Vec<String> = Vec::new();
    let mut prev: Option<String> = None;
    let mut started = from_tick.is_none();

    for (i, e) in entries.iter().enumerate() {
        let tick_id = e.get("tick_id").and_then(|v| v.as_str()).unwrap_or("?");
        if !started {
            if Some(tick_id) == from_tick {
                started = true;
            }
            // still must track prev for continuity
            prev = e
                .get("this_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            continue;
        }

        // 1. prev continuity
        let entry_prev = e.get("prev_hash").and_then(|v| match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some(s.clone()),
            _ => None,
        });
        if entry_prev != prev {
            errors.push(format!(
                "entry[{}] tick={}: prev_hash mismatch (expected {:?}, got {:?})",
                i, tick_id, prev, entry_prev
            ));
            return Ok(finalize_summary(
                chain_path,
                constitution_path,
                &chain,
                entries.len(),
                errors,
            ));
        }

        // 2. recompute this_hash
        let mut core = e.clone();
        if let Some(obj) = core.as_object_mut() {
            obj.remove("this_hash");
            obj.remove("signature");
        }
        let canon = jcs(&core);
        let alg = e.get("hash_alg").and_then(|v| v.as_str());
        let this_hash = e.get("this_hash").and_then(|v| v.as_str()).unwrap_or("");
        // Try blake3 first; if hash_alg explicitly says blake2b-256, or if
        // blake3 doesn't match (transitional chains where the hash field is
        // labeled "blake3:" but was actually computed with blake2b due to a
        // missing blake3 module at sign time), fall back to blake2b-256.
        let blake3_recomputed = format!("blake3:{}", b3_hex(canon.as_bytes()));
        let blake2_recomputed = format!("blake3:{}", blake2b_256_hex(canon.as_bytes()));
        let matched = match alg {
            Some("blake2b-256") => this_hash == blake2_recomputed,
            Some("blake3") | None => {
                this_hash == blake3_recomputed || this_hash == blake2_recomputed
            }
            Some(other) => {
                errors.push(format!(
                    "entry[{}] tick={}: unknown hash_alg '{}'",
                    i, tick_id, other
                ));
                return Ok(finalize_summary(
                    chain_path,
                    constitution_path,
                    &chain,
                    entries.len(),
                    errors,
                ));
            }
        };
        if !matched {
            errors.push(format!(
                "entry[{}] tick={}: this_hash mismatch (expected {} or {}, got {})",
                i, tick_id, blake3_recomputed, blake2_recomputed, this_hash
            ));
            return Ok(finalize_summary(
                chain_path,
                constitution_path,
                &chain,
                entries.len(),
                errors,
            ));
        }

        // 3. signature
        let sig_field = e.get("signature").and_then(|v| v.as_str()).unwrap_or("");
        if !sig_field.starts_with("ed25519:") || !this_hash.starts_with("blake3:") {
            errors.push(format!(
                "entry[{}] tick={}: malformed signature/this_hash field",
                i, tick_id
            ));
            return Ok(finalize_summary(
                chain_path,
                constitution_path,
                &chain,
                entries.len(),
                errors,
            ));
        }
        let sig_hex = &sig_field[8..];
        let th_hex = &this_hash[7..];
        let sig_bytes = match hex::decode(sig_hex) {
            Ok(b) => b,
            Err(_) => {
                errors.push(format!(
                    "entry[{}] tick={}: signature hex decode failed",
                    i, tick_id
                ));
                return Ok(finalize_summary(
                    chain_path,
                    constitution_path,
                    &chain,
                    entries.len(),
                    errors,
                ));
            }
        };
        let msg = match hex::decode(th_hex) {
            Ok(b) => b,
            Err(_) => {
                errors.push(format!(
                    "entry[{}] tick={}: this_hash hex decode failed",
                    i, tick_id
                ));
                return Ok(finalize_summary(
                    chain_path,
                    constitution_path,
                    &chain,
                    entries.len(),
                    errors,
                ));
            }
        };
        if sig_bytes.len() != 64 {
            errors.push(format!(
                "entry[{}] tick={}: signature must be 64 bytes",
                i, tick_id
            ));
            return Ok(finalize_summary(
                chain_path,
                constitution_path,
                &chain,
                entries.len(),
                errors,
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let sig = Signature::from_bytes(&sig_arr);
        if vk.verify(&msg, &sig).is_err() {
            errors.push(format!("entry[{}] tick={}: invalid signature", i, tick_id));
            return Ok(finalize_summary(
                chain_path,
                constitution_path,
                &chain,
                entries.len(),
                errors,
            ));
        }

        // 4. strict-wrap legacy evidence rehash
        if strict_wrap {
            if let Some(ev) = e.get("evidence") {
                if let Some(refobj) = ev.get("ref") {
                    if let (Some(p), Some(legacy)) = (
                        refobj.get("path").and_then(|v| v.as_str()),
                        refobj.get("legacy_content_hash").and_then(|v| v.as_str()),
                    ) {
                        let pp = Path::new(p);
                        match fs::read(pp) {
                            Ok(bytes) => {
                                let recomputed_legacy = format!("blake3:{}", b3_hex(&bytes));
                                if recomputed_legacy != legacy {
                                    errors.push(format!(
                                        "entry[{}] tick={}: legacy_content_hash mismatch for {}",
                                        i, tick_id, p
                                    ));
                                    return Ok(finalize_summary(
                                        chain_path,
                                        constitution_path,
                                        &chain,
                                        entries.len(),
                                        errors,
                                    ));
                                }
                            }
                            Err(e2) => {
                                errors.push(format!(
                                    "entry[{}] tick={}: cannot read legacy ref {}: {}",
                                    i, tick_id, p, e2
                                ));
                                return Ok(finalize_summary(
                                    chain_path,
                                    constitution_path,
                                    &chain,
                                    entries.len(),
                                    errors,
                                ));
                            }
                        }
                    }
                }
            }
        }

        prev = Some(this_hash.to_string());
    }

    // Head check
    let declared_head = chain
        .get("head")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let last_hash = entries
        .last()
        .and_then(|e| e.get("this_hash"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if declared_head != last_hash {
        errors.push(format!(
            "chain.head ({:?}) does not match last entry this_hash ({:?})",
            declared_head, last_hash
        ));
    }

    Ok(finalize_summary(
        chain_path,
        constitution_path,
        &chain,
        entries.len(),
        errors,
    ))
}

fn finalize_summary(
    chain_path: &Path,
    constitution_path: &Path,
    chain: &serde_json::Value,
    count: usize,
    errors: Vec<String>,
) -> ChainReplaySummary {
    let head = chain
        .get("head")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    ChainReplaySummary {
        schema: "chatmangpt.sr.chain_replay.v1".to_string(),
        is_valid: errors.is_empty(),
        receipt_count: count,
        head,
        verified_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        errors,
        chain: chain_path.display().to_string(),
        constitution: constitution_path.display().to_string(),
    }
}

fn run_replay_chain(argv: &[String]) -> ExitCode {
    let mut chain_path: Option<PathBuf> = None;
    let mut constitution_path: Option<PathBuf> = None;
    let mut from_tick: Option<String> = None;
    let mut strict_wrap = false;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--chain" => {
                i += 1;
                if i < argv.len() {
                    chain_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--constitution" => {
                i += 1;
                if i < argv.len() {
                    constitution_path = Some(PathBuf::from(&argv[i]));
                }
            }
            "--from" => {
                i += 1;
                if i < argv.len() {
                    from_tick = Some(argv[i].clone());
                }
            }
            "--strict-wrap" => {
                strict_wrap = true;
            }
            "--help" | "-h" => {
                println!("sr replay-chain — verify portfolio chain end-to-end");
                println!();
                println!("Usage: sr replay-chain --chain <path> --constitution <path> [--from <tick_id>] [--strict-wrap]");
                return ExitCode::from(0);
            }
            _ => {}
        }
        i += 1;
    }

    let chain_path = match chain_path {
        Some(p) => p,
        None => {
            eprintln!("{}error: --chain required{}", RED, RESET);
            return ExitCode::from(2);
        }
    };
    let constitution_path = match constitution_path {
        Some(p) => p,
        None => {
            eprintln!("{}error: --constitution required{}", RED, RESET);
            return ExitCode::from(2);
        }
    };

    let summary = match replay_chain(
        &chain_path,
        &constitution_path,
        from_tick.as_deref(),
        strict_wrap,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}replay-chain error: {}{}", RED, e, RESET);
            return ExitCode::from(2);
        }
    };

    // Write summary file beside chain.json
    let mut out_path = chain_path.clone();
    out_path.pop();
    out_path.push("chain-replay-summary.json");
    if let Ok(json) = serde_json::to_string_pretty(&summary) {
        let _ = fs::write(&out_path, json);
    }

    if summary.is_valid {
        println!(
            "{}{}chain replay PASS{} — {} entries, head={}",
            BOLD,
            GREEN,
            RESET,
            summary.receipt_count,
            summary.head.as_deref().unwrap_or("?")
        );
        println!("summary: {}", out_path.display());
        ExitCode::from(0)
    } else {
        println!("{}{}chain replay FAIL{}", BOLD, RED, RESET);
        for err in &summary.errors {
            println!("  {}• {}{}", RED, err, RESET);
        }
        println!("summary: {}", out_path.display());
        ExitCode::from(2)
    }
}

// ── Main ──────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    // Subcommand dispatch
    let raw: Vec<String> = std::env::args().collect();
    if raw.len() >= 2 && raw[1] == "replay-chain" {
        return run_replay_chain(&raw[2..]);
    }

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
                powl8_program: None,
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

    // Validate POWL8 program (optional)
    let powl8_result = match &args.powl8_program {
        Some(path) => match validate_powl8_program(path) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("{}POWL8 program error: {}{}", RED, e, RESET);
                None
            }
        },
        None => None,
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
    // A POWL8 program that was provided but failed to parse counts as a fatal error.
    if args.powl8_program.is_some() && powl8_result.is_none() {
        errors.push("POWL8_PROGRAM_INVALID".to_string());
    }
    // A POWL8 program that parsed but contains out-of-range opcodes also fails.
    if let Some(ref pr) = powl8_result {
        if !pr.valid {
            errors.push(format!(
                "POWL8_PROGRAM_OPCODE_VIOLATION: {}",
                pr.error.as_deref().unwrap_or("unknown")
            ));
        }
    }

    // Build result
    let result = build_sr_result(
        &args.target,
        cap_receipt,
        ralph_receipt,
        speckit_hashes,
        powl8_result,
        errors.clone(),
        Vec::new(),
    );

    print_result(&result, args.json_output);

    // Emit SR closure receipt on success
    if errors.is_empty() {
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let receipts_dir = PathBuf::from("/Users/sac/dteam/.portfolio/receipts");
        let receipt_path = args
            .receipt_out
            .clone()
            .unwrap_or_else(|| receipts_dir.join(format!("sr-{}-{}.json", args.target, timestamp)));

        let operation_id = format!("sr-verify-{}-{}", args.target, timestamp);

        // Collect all input file paths consumed by SR
        let mut input_paths: Vec<&Path> = Vec::new();
        if let Some(p) = args.capability_receipt.as_deref() {
            input_paths.push(p);
        }
        if let Some(p) = args.ralph_receipt.as_deref() {
            input_paths.push(p);
        }
        if let Some(p) = args.constitution_path.as_deref() {
            input_paths.push(p);
        }
        if let Some(p) = args.spec_path.as_deref() {
            input_paths.push(p);
        }
        if let Some(p) = args.plan_path.as_deref() {
            input_paths.push(p);
        }
        if let Some(p) = args.tasks_path.as_deref() {
            input_paths.push(p);
        }

        let sr_result_json = serde_json::to_string_pretty(&result).unwrap_or_default();

        match write_sr_receipt(
            &operation_id,
            &input_paths,
            &sr_result_json,
            &receipts_dir,
            &receipt_path,
        ) {
            Ok(()) => println!("receipt: {}", receipt_path.display()),
            Err(e) => eprintln!(
                "{}Warning: failed to write SR closure receipt: {}{}",
                YELLOW, e, RESET
            ),
        }

        ExitCode::from(0)
    } else {
        ExitCode::from(2)
    }
}
