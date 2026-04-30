//! SPR canonical: Mark-and-Chicken admissibility test.
//!
//! Demonstrates that the PackAdmission gate REFUSES non-admissible inputs at
//! the field boundary — the chicken does not get through. The soft fitness
//! ranking in `bitmask_replay::ReplayResult` is unaffected (different channel).

use dteam::autonomic::{AutonomicEvent, AutonomicState};
use dteam::breeds::BreedKind;
use dteam::pack_admission::{AdmissibilityVerdict, MarkChickenVerdict, PackAdmission};
use std::time::SystemTime;

fn ev(payload: &str) -> AutonomicEvent {
    AutonomicEvent {
        source: "mark-chicken-test".into(),
        payload: payload.into(),
        timestamp: SystemTime::now(),
    }
}

fn healthy_state() -> AutonomicState {
    use dteam::autonomic::types::PackPosture;
    AutonomicState {
        process_health: 0.9,
        throughput: 1.0,
        conformance_score: 0.9,
        drift_detected: false,
        active_cases: 1,
        field_elevation: 0.0,
        pack_posture: PackPosture::Nominal,
    }
}

#[test]
fn case_a_conformant_event_admitted() {
    let pack = PackAdmission::canonical();
    let verdict: MarkChickenVerdict = pack.accept_pack(&ev("normal"), &healthy_state());
    assert_eq!(verdict, AdmissibilityVerdict::Admitted);
}

#[test]
fn case_b_oversized_payload_refused_by_dachshund() {
    let pack = PackAdmission::canonical();
    let huge = "x".repeat(5000);
    match pack.accept_pack(&ev(&huge), &healthy_state()) {
        AdmissibilityVerdict::Refused { failing_breed, reason } => {
            assert_eq!(
                failing_breed,
                BreedKind::Dachshund,
                "expected Dachshund refusal, reason: {}",
                reason
            );
        }
        v => panic!("expected Refused, got {:?}", v),
    }
}

#[test]
fn case_c_drift_state_refused_by_detector() {
    let pack = PackAdmission::canonical();
    let mut state = healthy_state();
    state.drift_detected = true;
    match pack.accept_pack(&ev("normal"), &state) {
        AdmissibilityVerdict::Refused { failing_breed, .. } => {
            assert_eq!(failing_breed, BreedKind::Detector);
        }
        v => panic!("expected Refused due to drift, got {:?}", v),
    }
}

#[test]
fn chicken_does_not_get_through() {
    // Compose multiple violations: low conformance + drift + oversized payload.
    // First failing breed (per pack ordering) determines the rejection reason.
    let pack = PackAdmission::canonical();
    let mut state = healthy_state();
    state.conformance_score = 0.1;
    state.drift_detected = true;
    let huge = "y".repeat(5000);

    let verdict = pack.accept_pack(&ev(&huge), &state);
    assert!(
        matches!(verdict, AdmissibilityVerdict::Refused { .. }),
        "chicken made it past Mark: {:?}",
        verdict
    );

    // Soft fitness pipelines (bitmask_replay::ReplayResult) are NOT consulted
    // here — admissibility is a sibling channel. Rank-by-fitness behavior is
    // unaffected; the chicken simply never reaches it.
}
