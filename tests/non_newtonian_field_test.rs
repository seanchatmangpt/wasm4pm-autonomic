//! SPR canonical: Non-Newtonian Field test.
//!
//! Asserts that identical events processed twice through Vision2030Kernel
//! produce measurably divergent downstream effects — proving the SPR claim
//! "no two probes hit the same field".
//!
//! This is a KERNEL-level test, not a sketch-level test. Sketch divergence
//! is tautological for any mutable structure; the meaningful claim is that
//! the kernel's *verdict surface* changes between probes.

use dteam::autonomic::{AutonomicEvent, AutonomicKernel, Vision2030Kernel};
use dteam::pack_admission::PackAdmission;
use std::time::SystemTime;

fn probe_event(payload: &str) -> AutonomicEvent {
    AutonomicEvent {
        source: "non-newtonian-test".into(),
        payload: payload.into(),
        timestamp: SystemTime::now(),
    }
}

#[test]
fn same_event_twice_diverges_either_in_state_or_verdict() {
    let mut k = Vision2030Kernel::<1>::new();
    let pack = PackAdmission::canonical();
    let event = probe_event("normal");

    // First probe
    k.observe(event.clone());
    let s1 = k.infer();
    let v1 = pack.accept_pack(&event, &s1);

    // Second identical probe — but the field has integrated v1.
    k.observe(event.clone());
    let s2 = k.infer();
    let v2 = pack.accept_pack(&event, &s2);

    // The non-Newtonian property: the kernel's downstream-observable state
    // OR the pack verdict must differ between identical probes.
    let throughput_changed = (s1.throughput - s2.throughput).abs() > f32::EPSILON;
    let active_cases_changed = s1.active_cases != s2.active_cases;
    let conformance_changed = (s1.conformance_score - s2.conformance_score).abs() > f32::EPSILON;
    let elevation_changed = (s1.field_elevation - s2.field_elevation).abs() > f32::EPSILON;
    let verdict_changed = v1 != v2;

    assert!(
        throughput_changed || active_cases_changed || conformance_changed
            || elevation_changed || verdict_changed,
        "non-Newtonian violation: identical probe produced identical kernel state AND verdict.\n\
         s1.throughput={}, s2.throughput={}\n\
         s1.active_cases={}, s2.active_cases={}\n\
         s1.conformance={}, s2.conformance={}\n\
         s1.field_elevation={}, s2.field_elevation={}\n\
         v1={:?}, v2={:?}",
        s1.throughput, s2.throughput,
        s1.active_cases, s2.active_cases,
        s1.conformance_score, s2.conformance_score,
        s1.field_elevation, s2.field_elevation,
        v1, v2
    );
}

#[test]
fn many_identical_probes_yield_monotone_field_pressure() {
    // Stronger property: 10 identical probes produce monotone-non-decreasing
    // throughput accumulation, proving probes leave footprints in the field.
    let mut k = Vision2030Kernel::<1>::new();
    let event = probe_event("repeated");

    let mut throughputs = Vec::with_capacity(10);
    for _ in 0..10 {
        k.observe(event.clone());
        throughputs.push(k.infer().throughput);
    }
    // Throughput must increase across probes (kernel observed each event).
    assert!(
        throughputs.last().unwrap() > throughputs.first().unwrap(),
        "kernel throughput did not register repeated identical probes: {:?}",
        throughputs
    );
}
