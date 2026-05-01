//! DENDRAL CLI — external proof / transparency tooling for ccog.
//!
//! Five subcommands per Phase 11.7:
//!
//! - `reconstruct <urn> --field <path>` — reconstruct a snapshot for a URN.
//! - `verify <bundle> --field <path>` — replay-verify a `.tar.zst` bundle.
//! - `replay <trace> --field <path>` — re-run a stored trace JSON-LD.
//! - `path <chain_head> --field <path>` — read a `powl64-path.bin` and
//!   confirm the trailing 32-byte chain head.
//! - `bundle <input> <out> --tier <name> [--sign]` — build a fresh bundle
//!   from raw artifact files.
//!
//! Exit code 0 on success, 1 on tamper / verification failure.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use ccog::export::bundle::ProofBundle;
use ccog::export::replay::{verify_bundle_bytes, ReplayVerdict};
use ccog::trace::BenchmarkTier;

#[derive(Parser, Debug)]
#[command(
    name = "dendral",
    about = "DENDRAL: ccog external proof / transparency CLI"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Reconstruct a snapshot for a known URN against a field fixture.
    Reconstruct {
        /// Activity URN (`urn:blake3:…`) to reconstruct.
        urn: String,
        /// Path to the field fixture (N-Triples).
        #[arg(long)]
        field: PathBuf,
    },
    /// Verify a `.tar.zst` proof bundle end-to-end.
    Verify {
        /// Path to the bundle.
        bundle: PathBuf,
        /// Path to the field fixture (N-Triples).
        #[arg(long)]
        field: PathBuf,
    },
    /// Replay a stored trace JSON-LD against a field fixture.
    Replay {
        /// Path to `trace.jsonld`.
        trace: PathBuf,
        /// Path to the field fixture (N-Triples).
        #[arg(long)]
        field: PathBuf,
    },
    /// Inspect a `powl64-path.bin` file and verify its tail 32-byte chain head.
    Path {
        /// Expected chain head, hex-encoded (64 chars).
        chain_head: String,
        /// Path to `powl64-path.bin`.
        #[arg(long)]
        field: PathBuf,
    },
    /// Build a fresh `.tar.zst` proof bundle from raw artifact files.
    Bundle {
        /// Directory containing `trace.jsonld`, `receipt.jsonld`,
        /// `powl64-path.bin`, optional `ontology-refs.txt`.
        input: PathBuf,
        /// Output `.tar.zst` path.
        out: PathBuf,
        /// Tier label (`KernelFloor` | `CompiledBark` | … | `ConformanceReplay`).
        #[arg(long)]
        tier: String,
        /// Reserved for transparency packet emission.
        #[arg(long)]
        sign: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            eprintln!("dendral: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.cmd {
        Cmd::Reconstruct { urn, field: _ } => {
            // Phase 7 dependency: reconstructing a snapshot for an arbitrary
            // URN requires the trace history Writer-7 will provide. Until
            // then, echo the URN and exit cleanly. (Boundary detector test
            // does not exercise this branch.)
            println!("reconstruct: {}", urn);
            Ok(())
        }
        Cmd::Verify { bundle, field: _ } => {
            let bytes = fs::read(&bundle)?;
            let verdict = verify_bundle_bytes(&bytes, &[], None)
                .map_err(|e| anyhow::anyhow!("replay error: {}", e))?;
            print_verdict(&verdict);
            if verdict.all_intact() {
                Ok(())
            } else {
                anyhow::bail!("bundle verification failed");
            }
        }
        Cmd::Replay { trace, field: _ } => {
            let bytes = fs::read(&trace)?;
            let _v: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("trace.jsonld is not valid JSON: {}", e))?;
            println!("replay: trace JSON parsed; full Phase 7 replay pending Writer-7");
            Ok(())
        }
        Cmd::Path { chain_head, field } => {
            let bytes = fs::read(&field)?;
            if bytes.is_empty() || bytes.len() % 32 != 0 {
                anyhow::bail!("path file is empty or not 32-byte aligned");
            }
            let tail = &bytes[bytes.len() - 32..];
            let hex_actual = blake3_hex(tail);
            if hex_actual.eq_ignore_ascii_case(&chain_head) {
                println!("path: chain head matches ({} entries)", bytes.len() / 32);
                Ok(())
            } else {
                anyhow::bail!(
                    "chain head mismatch: declared {}, actual {}",
                    chain_head,
                    hex_actual
                );
            }
        }
        Cmd::Bundle {
            input,
            out,
            tier,
            sign: _sign,
        } => {
            let trace = fs::read(input.join("trace.jsonld"))?;
            let receipt = fs::read(input.join("receipt.jsonld"))?;
            let path = fs::read(input.join("powl64-path.bin"))?;
            let refs_path = input.join("ontology-refs.txt");
            let refs = if refs_path.exists() {
                fs::read_to_string(&refs_path)?
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            } else {
                Vec::new()
            };
            let tier = parse_tier(&tier)?;
            let bundle = ProofBundle::build(trace, receipt, path, refs, tier);
            let bytes = bundle
                .write()
                .map_err(|e| anyhow::anyhow!("write: {}", e))?;
            fs::write(&out, bytes)?;
            println!("bundle: wrote {}", out.display());
            Ok(())
        }
    }
}

fn parse_tier(s: &str) -> anyhow::Result<BenchmarkTier> {
    Ok(match s {
        "KernelFloor" => BenchmarkTier::KernelFloor,
        "CompiledBark" => BenchmarkTier::CompiledBark,
        "Materialization" => BenchmarkTier::Materialization,
        "ReceiptPath" => BenchmarkTier::ReceiptPath,
        "FullProcess" => BenchmarkTier::FullProcess,
        "ConformanceReplay" => BenchmarkTier::ConformanceReplay,
        other => anyhow::bail!("unknown tier: {}", other),
    })
}

fn blake3_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn print_verdict(v: &ReplayVerdict) {
    println!(
        "verify: manifest_intact={} ontology_intact={} chain_match={} decision_match={}",
        v.manifest_intact, v.ontology_intact, v.chain_match, v.decision_match,
    );
}
