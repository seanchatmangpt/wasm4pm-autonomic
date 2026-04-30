//! End-to-end auditability pipeline test.
//!
//! Wires the three orphaned modules into a single demonstrable pipeline:
//!     `mycin::infer_fast` → `PredictionLogBuffer::log_prediction`
//!     → `compute_confusion_matrix` → `detect_drift`
//!     → `handle_drift_signal` → `RetrainingAction`.
//!
//! Backs §5.6 of `docs/COMPILED_COGNITION.md`: "auditability is structural,
//! not procedural."

use dteam::io::prediction_log::{PredictionLogBuffer, blake3_input_hash};
use dteam::ml::drift_detector::{compute_confusion_matrix, detect_drift, DriftSignal};
use dteam::ml::mycin::{infer_fast, RULES};
use dteam::ml::retraining_orchestrator::{handle_drift_signal, RetrainingAction};

/// Hash a fact bitmask deterministically for `input_hash` using BLAKE3-256.
fn hash_facts(facts: u64) -> [u8; 32] {
    blake3_input_hash(&facts.to_le_bytes())
}

#[test]
fn auditability_pipeline_mycin_drift_to_retrain() {
    const N: usize = 256;
    const TOTAL: usize = 128;
    const PROVENANCE_HASH_MYCIN_V1: u64 = 0x4D59_4349_4E5F_5631; // "MYCIN_V1"

    let log = PredictionLogBuffer::<N>::new(/*binary_version=*/ 1);

    // 128 deterministic MYCIN inferences over a small fact-mask sweep.
    let mut predictions: Vec<bool> = Vec::with_capacity(TOTAL);
    let mut observed: Vec<bool> = Vec::with_capacity(TOTAL);
    let mut tier_seq: Vec<u8> = Vec::with_capacity(TOTAL);

    for i in 0..TOTAL as u64 {
        let facts = (i * 0x1F1F_1F1F) & 0x0000_0000_0000_FFFF; // 16-bit fact slice
        let conclusions = infer_fast(facts, &RULES);
        // Decision: any organism inferred?
        let decision = conclusions != 0;

        let input_hash = hash_facts(facts);
        log.log_prediction(
            input_hash,
            /*timestamp_us=*/ 0,
            decision,
            /*tier_fired=*/ 0,
            PROVENANCE_HASH_MYCIN_V1,
        );

        predictions.push(decision);
        // Inject drift on the second half: flip 32 of the last 64 observed labels.
        let drift_flip = i >= 64 && (i % 2 == 0);
        observed.push(decision ^ drift_flip);
        tier_seq.push(0);
    }

    // 1) Log captured every prediction.
    assert_eq!(log.len(), TOTAL, "log must hold all 128 predictions");

    // 2) Confusion metrics computed honestly.
    let metrics = compute_confusion_matrix(&predictions, &observed);
    assert_eq!(metrics.total() as usize, TOTAL);
    assert!(
        metrics.accuracy() < 0.95,
        "injected drift must drop accuracy below 0.95 (got {})",
        metrics.accuracy()
    );

    // 3) Drift detector emits a non-Healthy signal at baseline 0.95.
    let signal = detect_drift(&metrics, &predictions, &observed, &tier_seq, /*baseline_accuracy=*/ 0.95, &[]);
    assert!(
        signal.needs_retraining(),
        "drift signal must indicate retraining needed; got {:?}",
        signal
    );
    assert_ne!(signal, DriftSignal::Healthy);

    // 4) Orchestrator routes the signal to a non-Continue action.
    let action = handle_drift_signal(signal);
    assert_ne!(
        action,
        RetrainingAction::Continue,
        "non-healthy signal must produce a retraining action"
    );
    assert!(
        matches!(
            action,
            RetrainingAction::CreateRetrainingTicket | RetrainingAction::ApprovedRetrainThenRebuild
        ),
        "action must be a known retraining variant; got {:?}",
        action
    );
}
