//! Integration tests for Vision2030Kernel and ocpm::StreamingOcDfg.
//!
//! T003: Validates that Vision2030Kernel::run_cycle produces real (non-sentinel) hashes
//! and that StreamingOcDfg correctly tracks edge and object counts.

use dteam::autonomic::{AutonomicEvent, AutonomicKernel, Vision2030Kernel};
use dteam::ocpm::StreamingOcDfg;
use std::time::SystemTime;

// ── Test 1: Vision2030Kernel run_cycle produces a real hash ──────────────────

#[test]
fn vision2030_kernel_run_cycle_produces_real_hash() {
    let mut kernel: Vision2030Kernel<1> = Vision2030Kernel::new();

    // Construct a synthetic event with a payload that triggers the "start" path
    // so the activity is recognized (observe maps it to idx=0).
    let event = AutonomicEvent {
        source: "integration-test".to_string(),
        payload: "start obj creates order".to_string(),
        timestamp: SystemTime::now(),
    };

    let results = kernel.run_cycle(event);

    // run_cycle must produce at least one result (health=1.0, conformance=1.0 initially)
    assert!(
        !results.is_empty(),
        "run_cycle must produce at least one AutonomicResult"
    );

    let result = &results[0];

    // manifest_hash must NOT be the old hardcoded sentinel 0x2030_ABCD
    assert_ne!(
        result.manifest_hash, 0x2030_ABCD,
        "manifest_hash must be a real hash, not the placeholder sentinel 0x2030_ABCD"
    );

    // execution_latency_ms must be plausible: > 0 is not strictly required (sub-ms is valid)
    // but must be < 10000 ms (no runaway execution)
    assert!(
        result.execution_latency_ms < 10_000,
        "execution_latency_ms={} exceeds 10s; runaway execution detected",
        result.execution_latency_ms
    );
}

// ── Test 2: StreamingOcDfg ingests events and reports non-zero counts ─────────

#[test]
fn streaming_oc_dfg_ingests_events_and_reports_counts() {
    // Use small power-of-two cache sizes to keep the test lightweight.
    let mut dfg: StreamingOcDfg<16, 64> = StreamingOcDfg::new();

    // Define 2 activity hash values (simulating "register" and "approve")
    let activity_register: u64 = 0xAAAA_1111_BBBB_0001;
    let activity_approve: u64 = 0xAAAA_1111_BBBB_0002;

    // Define 3 objects (order, item-a, item-b) with distinct hashes
    let obj_order: u64 = 0x0001_0001_0001_0001;
    let obj_item_a: u64 = 0x0002_0002_0002_0002;
    let obj_item_b: u64 = 0x0003_0003_0003_0003;

    let type_order: u64 = 0x1000;
    let type_item: u64 = 0x2000;
    let qualifier_creates: u64 = 0xC1;
    let qualifier_reads: u64 = 0xE1;

    // Event 1: "register" touches order + item-a (2 objects)
    dfg.observe_event(
        activity_register,
        &[
            (obj_order, type_order, qualifier_creates),
            (obj_item_a, type_item, qualifier_creates),
        ],
    );

    // Event 2: "register" touches item-b (1 object)
    dfg.observe_event(
        activity_register,
        &[(obj_item_b, type_item, qualifier_creates)],
    );

    // Event 3: "approve" touches order (follows "register" for this object → creates an edge)
    dfg.observe_event(
        activity_approve,
        &[(obj_order, type_order, qualifier_reads)],
    );

    // Event 4: "approve" touches item-a (follows "register" → edge)
    dfg.observe_event(
        activity_approve,
        &[(obj_item_a, type_item, qualifier_reads)],
    );

    // Event 5: "approve" touches item-b (follows "register" → edge)
    dfg.observe_event(
        activity_approve,
        &[(obj_item_b, type_item, qualifier_reads)],
    );

    // Assert: binding_frequencies must have at least one non-zero entry.
    // Every observe_event call unconditionally increments binding_frequencies.
    let total_bindings: u32 = dfg.binding_frequencies.iter().sum();
    assert!(
        total_bindings > 0,
        "binding_frequencies must be non-zero after ingesting 5 events"
    );

    // Assert: edge_frequencies must have at least one non-zero entry.
    // Edges are recorded when an object has a prior activity (prev_activity != 0).
    // Events 3, 4, and 5 each follow a prior "register" for the same object.
    let total_edges: u32 = dfg.edge_frequencies.iter().sum();
    assert!(
        total_edges > 0,
        "edge_frequencies must be non-zero: events 3-5 each produce a register→approve edge"
    );

    // Assert: at least 3 distinct object slots were written.
    // last_activity_per_obj[slot] is set to the activity hash on each observe_event call.
    let active_objects = dfg
        .last_activity_per_obj
        .iter()
        .filter(|&&v| v != 0)
        .count();
    assert!(
        active_objects >= 3,
        "expected ≥3 active object slots (order, item-a, item-b), got {}",
        active_objects
    );
}
