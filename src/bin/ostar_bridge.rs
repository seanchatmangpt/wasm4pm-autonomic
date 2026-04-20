//! ostar_bridge — JSON RPC bridge to dteam process intelligence kernel.
//! Minimal stub that echoes back JSON responses for protocol validation.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::io::Read;

const MAX_PAYLOAD_SIZE: usize = 2000;

fn main() -> Result<()> {
    let mut input = String::new();
    let read_bytes = std::io::stdin()
        .take(MAX_PAYLOAD_SIZE as u64)
        .read_to_string(&mut input)
        .context("Failed to read stdin")?;

    if read_bytes >= MAX_PAYLOAD_SIZE {
        return Err(anyhow!(
            "Payload exceeds maximum size of {} characters",
            MAX_PAYLOAD_SIZE
        ));
    }

    if input.trim().is_empty() {
        return Ok(());
    }

    // Basic sanitization: Ensure no control characters that could be used for injection
    if input.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return Err(anyhow!("Payload contains illegal control characters"));
    }

    let cmd: Value = serde_json::from_str(&input).context("Invalid JSON input")?;

    let op = cmd
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing 'op' field"))?;

    let result = match op {
        "ping" => handle_ping(),
        "discover" => handle_discover(&cmd),
        "conform" => handle_conform(&cmd),
        "autonomic" => handle_autonomic(&cmd),
        _ => Err(anyhow!("Unknown op: {}", op)),
    };

    match result {
        Ok(out) => {
            println!("{}", out);
            Ok(())
        }
        Err(e) => {
            let err_json = json!({
                "ok": false,
                "error": e.to_string()
            });
            println!("{}", err_json);
            std::process::exit(1);
        }
    }
}

fn handle_ping() -> Result<String> {
    Ok(json!({
        "ok": true,
        "message": "pong"
    })
    .to_string())
}

fn handle_discover(_cmd: &Value) -> Result<String> {
    // Stub: In production, call dteam::Engine::run() here.
    // For MVP, echo back a valid response.
    Ok(json!({
        "ok": true,
        "petri_net": {
            "places_count": 0,
            "transitions_count": 0,
            "arcs_count": 0,
        },
        "manifest": {
            "input_log_hash": 0,
            "model_canonical_hash": 0,
            "mdl_score": 0.0,
            "k_tier": "K256",
            "latency_ns": 0,
        },
    })
    .to_string())
}

fn handle_conform(_cmd: &Value) -> Result<String> {
    // Stub: In production, call dteam::conformance::token_replay() here.
    Ok(json!({
        "ok": true,
        "overall_fitness": 0.9,
        "cases": [],
    })
    .to_string())
}

fn handle_autonomic(_cmd: &Value) -> Result<String> {
    // Stub: In production, call DefaultKernel::run_cycle() here.
    Ok(json!({
        "ok": true,
        "results": [{
            "success": true,
            "execution_latency_ms": 1,
            "manifest_hash": 0
        }]
    })
    .to_string())
}
