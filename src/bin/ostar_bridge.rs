//! ostar_bridge — JSON RPC bridge to dteam process intelligence kernel.

use anyhow::{anyhow, Context, Result};
use dteam::autonomic::{AutonomicEvent, AutonomicKernel, DefaultKernel};
use dteam::conformance::token_replay;
use dteam::dteam::orchestration::{EngineBuilder, EngineResult};
use dteam::models::petri_net::PetriNet;
use dteam::models::EventLog;
use serde_json::{json, Value};
use std::io::Read;
use std::time::{Instant, SystemTime};

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
        "discover_powl" => handle_discover_powl(&cmd),
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

fn handle_discover(cmd: &Value) -> Result<String> {
    let log_value = cmd
        .get("log")
        .ok_or_else(|| anyhow!("Missing 'log' field"))?
        .clone();
    let log: EventLog =
        serde_json::from_value(log_value).context("Failed to parse 'log' as EventLog")?;

    let engine = EngineBuilder::new().build();
    let result = engine.run(&log);

    match result {
        EngineResult::Success(net, manifest) => Ok(json!({
            "ok": true,
            "petri_net": {
                "places_count": net.places.len(),
                "transitions_count": net.transitions.len(),
                "arcs_count": net.arcs.len(),
            },
            "manifest": {
                "H(L)": manifest.input_log_hash,
                "pi": manifest.action_sequence,
                "H(N)": manifest.model_canonical_hash,
                "integrity_hash": manifest.model_canonical_hash,
                "mdl_score": manifest.mdl_score,
                "k_tier": manifest.k_tier,
                "latency_ns": manifest.latency_ns,
            },
        })
        .to_string()),
        EngineResult::PartitionRequired {
            required,
            configured,
        } => Err(anyhow!(
            "Partition required: log needs {} activities but engine configured for {}",
            required,
            configured
        )),
        EngineResult::BoundaryViolation { activity } => Err(anyhow!(
            "Boundary violation: activity '{}' not in ontology",
            activity
        )),
    }
}

fn handle_conform(cmd: &Value) -> Result<String> {
    let log_value = cmd
        .get("log")
        .ok_or_else(|| anyhow!("Missing 'log' field"))?
        .clone();
    let log: EventLog =
        serde_json::from_value(log_value).context("Failed to parse 'log' as EventLog")?;

    let model_value = cmd
        .get("model")
        .ok_or_else(|| anyhow!("Missing 'model' field"))?
        .clone();
    let petri_net: PetriNet =
        serde_json::from_value(model_value).context("Failed to parse 'model' as PetriNet")?;

    let results = token_replay(&log, &petri_net);

    let overall_fitness = if results.is_empty() {
        1.0
    } else {
        results.iter().map(|r| r.fitness).sum::<f64>() / results.len() as f64
    };

    let cases: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "case_id": r.case_id,
                "fitness": r.fitness,
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "overall_fitness": overall_fitness,
        "cases": cases,
    })
    .to_string())
}

fn handle_discover_powl(cmd: &Value) -> Result<String> {
    use dteam::discovery::powl::discover_powl;
    use dteam::powl::conversion::to_petri_net::powl_to_wf_net;

    let log_value = cmd
        .get("log")
        .ok_or_else(|| anyhow!("Missing 'log' field"))?
        .clone();
    let log: EventLog =
        serde_json::from_value(log_value).context("Failed to parse 'log' as EventLog")?;

    let powl_model =
        discover_powl(&log.traces).map_err(|e| anyhow!("POWL discovery failed: {}", e))?;
    let net = powl_to_wf_net(&powl_model.root);

    Ok(json!({
        "ok": true,
        "petri_net": {
            "places_count": net.places.len(),
            "transitions_count": net.transitions.len(),
            "arcs_count": net.arcs.len(),
        },
    })
    .to_string())
}

fn handle_autonomic(cmd: &Value) -> Result<String> {
    use dteam::utils::dense_kernel::fnv1a_64;

    let mut kernel = DefaultKernel::new();

    let source = cmd
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("ostar_bridge")
        .to_string();
    let payload = cmd
        .get("payload")
        .and_then(Value::as_str)
        .unwrap_or("autonomic_cycle")
        .to_string();

    let event = AutonomicEvent {
        source: source.clone(),
        payload: payload.clone(),
        timestamp: SystemTime::now(),
    };

    let start = Instant::now();
    let results = kernel.run_cycle(event);
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let result_values: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "success": r.success,
                "execution_latency_ms": elapsed_ms,
                "manifest_hash": r.manifest_hash,
                "guarded": false,
            })
        })
        .collect();

    // If no results were produced (guards prevented execution), return a success with measured latency
    // Compute deterministic hash via fnv1a_64 on source+payload
    let guard_hash = fnv1a_64(format!("{}{}", source, payload).as_bytes());
    let output = if result_values.is_empty() {
        vec![json!({
            "success": true,
            "execution_latency_ms": elapsed_ms,
            "manifest_hash": guard_hash,
            "guarded": true,
        })]
    } else {
        result_values
    };

    Ok(json!({
        "ok": true,
        "results": output,
    })
    .to_string())
}
