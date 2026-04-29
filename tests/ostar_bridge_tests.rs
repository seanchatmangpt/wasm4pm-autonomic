//! Integration tests for ostar_bridge JSON-RPC handlers.
//! Each test invokes the binary via std::process::Command and asserts on real computed values.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

/// Minimal synthetic EventLog with 2 traces and activities A→B→C / A→C.
fn minimal_xes_json() -> Value {
    json!({
        "traces": [
            {
                "id": "case-1",
                "events": [
                    {"attributes": [{"key": "concept:name", "value": {"type": "String", "content": "A"}}]},
                    {"attributes": [{"key": "concept:name", "value": {"type": "String", "content": "B"}}]},
                    {"attributes": [{"key": "concept:name", "value": {"type": "String", "content": "C"}}]}
                ],
                "attributes": []
            },
            {
                "id": "case-2",
                "events": [
                    {"attributes": [{"key": "concept:name", "value": {"type": "String", "content": "A"}}]},
                    {"attributes": [{"key": "concept:name", "value": {"type": "String", "content": "C"}}]}
                ],
                "attributes": []
            }
        ],
        "attributes": []
    })
}

/// Run the ostar_bridge binary with the given JSON payload, return parsed response.
fn run_bridge(payload: Value) -> (Value, bool) {
    let input = serde_json::to_string(&payload).expect("Failed to serialize payload");

    let mut child = Command::new("cargo")
        .args(["run", "--bin", "ostar_bridge", "-q"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn ostar_bridge");

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for ostar_bridge");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let response: Value =
        serde_json::from_str(stdout.trim()).unwrap_or(json!({"ok": false, "error": stdout}));
    (response, output.status.success())
}

#[test]
fn discover_returns_real_petri_net() {
    let payload = json!({
        "op": "discover",
        "log": minimal_xes_json()
    });

    let (response, exited_ok) = run_bridge(payload);

    assert!(exited_ok, "ostar_bridge exited with error: {}", response);
    assert_eq!(
        response["ok"], true,
        "response ok must be true: {}",
        response
    );

    // The engine may produce nets with 0 places (topology varies by RL policy)
    // but must always produce at least one transition when activities are present.
    let transitions_count = response["petri_net"]["transitions_count"]
        .as_u64()
        .expect("transitions_count must be a number");
    assert!(
        transitions_count > 0,
        "transitions_count must be > 0, got {} — discovery returned empty net",
        transitions_count
    );

    let mdl_score = response["manifest"]["mdl_score"]
        .as_f64()
        .expect("mdl_score must be a number");
    assert!(
        mdl_score > 0.0,
        "mdl_score must be > 0.0, got {} — discovery returned zero score",
        mdl_score
    );
}

#[test]
fn conform_returns_real_fitness() {
    // First discover to get a real petri net model
    let discover_payload = json!({
        "op": "discover",
        "log": minimal_xes_json()
    });
    let (discover_response, _) = run_bridge(discover_payload);
    assert_eq!(
        discover_response["ok"], true,
        "discover must succeed: {}",
        discover_response
    );

    // Build a minimal PetriNet that matches the A→B→C / A→C traces.
    // initial_marking and final_markings use the PackedKeyTable serialization format.
    let petri_net = json!({
        "places": [
            {"id": "start"},
            {"id": "p1"},
            {"id": "p2"},
            {"id": "end"}
        ],
        "transitions": [
            {"id": "tA", "label": "A", "is_invisible": null},
            {"id": "tB", "label": "B", "is_invisible": null},
            {"id": "tC", "label": "C", "is_invisible": null}
        ],
        "arcs": [
            {"from": "start", "to": "tA", "weight": 1},
            {"from": "tA", "to": "p1", "weight": 1},
            {"from": "p1", "to": "tB", "weight": 1},
            {"from": "tB", "to": "p2", "weight": 1},
            {"from": "p1", "to": "tC", "weight": 1},
            {"from": "p2", "to": "tC", "weight": 1},
            {"from": "tC", "to": "end", "weight": 1}
        ],
        "initial_marking": {"entries": []},
        "final_markings": []
    });

    let conform_payload = json!({
        "op": "conform",
        "log": minimal_xes_json(),
        "model": petri_net
    });

    let (response, exited_ok) = run_bridge(conform_payload);

    assert!(exited_ok, "ostar_bridge exited with error: {}", response);
    assert_eq!(
        response["ok"], true,
        "response ok must be true: {}",
        response
    );

    let overall_fitness = response["overall_fitness"]
        .as_f64()
        .expect("overall_fitness must be a number");

    assert!(
        overall_fitness > 0.0 && overall_fitness <= 1.0,
        "overall_fitness must be in (0.0, 1.0], got {}",
        overall_fitness
    );

    // The fabricated stub always returned exactly 0.9 — real token replay must differ
    assert_ne!(
        overall_fitness, 0.9f64,
        "overall_fitness must not be the fabricated constant 0.9 — token replay is not running"
    );
}

#[test]
fn discover_powl_returns_valid_result() {
    let payload = json!({
        "op": "discover_powl",
        "log": minimal_xes_json()
    });

    let (response, exited_ok) = run_bridge(payload);

    assert!(exited_ok, "ostar_bridge exited with error: {}", response);
    assert_eq!(
        response["ok"], true,
        "response ok must be true: {}",
        response
    );

    // The handle_discover_powl handler returns petri_net.{places_count, transitions_count, arcs_count}.
    // Assert transitions_count > 0 to confirm a valid result was returned.
    let transitions_count = response["transitions_count"]
        .as_u64()
        .expect("transitions_count must be a number");
    assert!(
        transitions_count > 0,
        "transitions_count must be > 0, got {} — POWL discovery returned empty net",
        transitions_count
    );
}

#[test]
fn autonomic_returns_measured_latency() {
    let payload = json!({
        "op": "autonomic"
    });

    let (response, exited_ok) = run_bridge(payload);

    assert!(exited_ok, "ostar_bridge exited with error: {}", response);
    assert_eq!(
        response["ok"], true,
        "response ok must be true: {}",
        response
    );

    let results = response["results"]
        .as_array()
        .expect("results must be an array");
    assert!(!results.is_empty(), "results array must not be empty");

    let first = &results[0];
    let execution_latency_ms = first["execution_latency_ms"]
        .as_u64()
        .expect("execution_latency_ms must be a non-negative integer");

    // Latency must be a real measured value (>= 0), not the fabricated constant 1
    assert_ne!(
        execution_latency_ms, 1u64,
        "execution_latency_ms must not be the fabricated constant 1 — DefaultKernel is not running"
    );

    // A real measured latency is >= 0 (trivially true for u64, but assert intent)
    assert!(
        execution_latency_ms < 60_000,
        "execution_latency_ms {} exceeds 60s — likely a bug",
        execution_latency_ms
    );
}
