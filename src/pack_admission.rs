//! SPR canonical: PackAdmission — Accept_pack(x, O*) = ⋀_i Breed_i(x, O*)
//!
//! Heterogeneous conjunction over structurally distinct breeds. Returns a
//! sibling channel `AdmissibilityVerdict` — does NOT replace soft fitness
//! semantics in `bitmask_replay::ReplayResult`. Soft ranking pipelines
//! (`pdc_combinator`, `automl_eval`) remain unchanged.
//!
//! Slogan: "You cannot talk chicken past Mark." See COMPILED_COGNITION.md §3.

use crate::autonomic::types::{AutonomicEvent, AutonomicState};
use crate::breeds::{Breed, BreedKind, Dachshund, Detector, Guardian, Herder, Recorder, Retriever, Watchdog};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissibilityVerdict {
    Admitted,
    Refused { failing_breed: BreedKind, reason: &'static str },
}

/// SPR alias for AdmissibilityVerdict; emphasizes the "chicken does not get
/// through" boundary semantic.
pub type MarkChickenVerdict = AdmissibilityVerdict;

/// PackAdmission with the canonical 7-breed roster.
/// Static dispatch via `&'static dyn Breed`; zero heap.
pub struct PackAdmission {
    breeds: &'static [&'static (dyn Breed + Sync)],
}

static GUARDIAN: Guardian = Guardian;
static DETECTOR: Detector = Detector;
static HERDER: Herder = Herder;
static WATCHDOG: Watchdog = Watchdog;
static RETRIEVER: Retriever = Retriever;
static RECORDER: Recorder = Recorder;
static DACHSHUND: Dachshund = Dachshund;

static CANONICAL_PACK: &[&(dyn Breed + Sync)] = &[
    &GUARDIAN, &DETECTOR, &HERDER, &WATCHDOG, &RETRIEVER, &RECORDER, &DACHSHUND,
];

impl PackAdmission {
    pub const fn canonical() -> Self {
        PackAdmission { breeds: CANONICAL_PACK }
    }

    /// Accept_pack(x, O*) = ⋀_i Breed_i(x, O*). Short-circuits on first refusal.
    pub fn accept_pack(&self, ev: &AutonomicEvent, st: &AutonomicState) -> AdmissibilityVerdict {
        for breed in self.breeds {
            if !breed.admit(ev, st) {
                return AdmissibilityVerdict::Refused {
                    failing_breed: breed.kind(),
                    reason: reason_for(breed.kind()),
                };
            }
        }
        AdmissibilityVerdict::Admitted
    }
}

fn reason_for(b: BreedKind) -> &'static str {
    match b {
        BreedKind::Guardian  => "structural envelope violated (low conformance)",
        BreedKind::Detector  => "drift detected — semantic field disturbance",
        BreedKind::Herder    => "behavioral fitness below threshold",
        BreedKind::Watchdog  => "process health critical",
        BreedKind::Retriever => "evidence retrieval load excessive",
        BreedKind::Recorder  => "recording channel inhibited",
        BreedKind::Dachshund => "payload depth probe-like — refuse",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn ev(p: &str) -> AutonomicEvent {
        AutonomicEvent { source: "t".into(), payload: p.into(), timestamp: SystemTime::now() }
    }
    fn healthy_state() -> AutonomicState {
        AutonomicState {
            process_health: 0.9, throughput: 1.0, conformance_score: 0.9,
            drift_detected: false, active_cases: 1, field_elevation: 0.0,
            pack_posture: crate::autonomic::types::PackPosture::Nominal,
        }
    }

    #[test]
    fn clean_input_admitted() {
        let pack = PackAdmission::canonical();
        let v = pack.accept_pack(&ev("normal"), &healthy_state());
        assert_eq!(v, AdmissibilityVerdict::Admitted);
    }

    #[test]
    fn drift_state_refused_by_detector() {
        let pack = PackAdmission::canonical();
        let mut s = healthy_state();
        s.drift_detected = true;
        match pack.accept_pack(&ev("normal"), &s) {
            AdmissibilityVerdict::Refused { failing_breed, .. } => {
                assert_eq!(failing_breed, BreedKind::Detector);
            }
            v => panic!("expected Refused, got {:?}", v),
        }
    }

    #[test]
    fn low_conformance_refused_by_guardian() {
        let pack = PackAdmission::canonical();
        let mut s = healthy_state();
        s.conformance_score = 0.2;
        match pack.accept_pack(&ev("normal"), &s) {
            AdmissibilityVerdict::Refused { failing_breed, .. } => {
                assert_eq!(failing_breed, BreedKind::Guardian);
            }
            v => panic!("expected Refused, got {:?}", v),
        }
    }

    #[test]
    fn oversized_payload_refused_by_dachshund() {
        let pack = PackAdmission::canonical();
        let big = "x".repeat(5000);
        match pack.accept_pack(&ev(&big), &healthy_state()) {
            AdmissibilityVerdict::Refused { failing_breed, .. } => {
                assert_eq!(failing_breed, BreedKind::Dachshund);
            }
            v => panic!("expected Refused, got {:?}", v),
        }
    }

    #[test]
    fn mark_chicken_alias_works() {
        let pack = PackAdmission::canonical();
        let v: MarkChickenVerdict = pack.accept_pack(&ev("ok"), &healthy_state());
        assert_eq!(v, AdmissibilityVerdict::Admitted);
    }
}
