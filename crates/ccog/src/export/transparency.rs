//! Transparency anchoring (Phase 11.6).
//!
//! Off by default. Enabled via the `transparency` Cargo feature, which
//! pulls in `ed25519-dalek` for signing. Packet shape is RFC6962-style;
//! ccog itself does **not** depend on `sigstore-rs`.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::powl64::ProjectionTarget;
use crate::runtime::conformance::{ConformanceReport, EvidenceLedger};

/// Submission packet — `chain_head` + `bundle_hash` + Ed25519 signature.
///
/// `signature` is split into two 32-byte halves (`signature_lo` /
/// `signature_hi`) so serde derives work on stable without the
/// `serde-big-array` dependency. Re-assemble via [`Self::signature_bytes`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionPacket {
    /// 32-byte chain head being anchored.
    pub chain_head: [u8; 32],
    /// 32-byte bundle hash (BLAKE3 of the `.tar.zst` bytes).
    pub bundle_hash: [u8; 32],
    /// Low 32 bytes of the 64-byte Ed25519 signature.
    pub signature_lo: [u8; 32],
    /// High 32 bytes of the 64-byte Ed25519 signature.
    pub signature_hi: [u8; 32],
    /// 32-byte Ed25519 verifying key.
    pub pubkey: [u8; 32],
}

/// Final Process-Mining Scorecard (Wil van der Aalst Report).
///
/// Reports the performance, conformance, and ecology metrics for a cognitive run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VanDerAalstScorecard {
    /// Case Throughput (total number of traces processed).
    pub case_throughput: f64,
    /// Fitness: Ratio of observed trace steps admissible in topology [0.0, 1.0].
    pub fitness: f64,
    /// Precision: Ratio of topology edges exercised by the ledger [0.0, 1.0].
    pub precision: f64,
    /// Alignment Cost: Aggregate penalty for fitness violations (moves-on-log).
    pub alignment_cost: f64,
    /// Unnecessary Externalization Rate: Ratio of steps requiring human-in-the-loop intervention.
    pub externalization_rate: f64,
    /// Drift Epoch: Cryptographic chain head where a significant policy or resource drift was detected.
    pub drift_epoch: u64,
}

impl VanDerAalstScorecard {
    /// Generate a scorecard from an evidence ledger and its conformance report.
    #[must_use]
    pub fn generate(ledger: &EvidenceLedger, report: &ConformanceReport) -> Self {
        let mut total_steps = 0;
        let mut hitl_steps = 0;
        let mut latest_drift_epoch = 0;

        for trace in &ledger.traces {
            total_steps += trace.cells.len();
            for cell in &trace.cells {
                if cell.projection_target == ProjectionTarget::Hitl {
                    hitl_steps += 1;
                }

                // Heuristic drift detection: identify the latest point of divergence.
                // We use chain_head as the epoch marker.
                if cell.chain_head > latest_drift_epoch
                    && (report.fitness < 1.0 || report.precision < 1.0)
                {
                    latest_drift_epoch = cell.chain_head;
                }
            }
        }

        let case_throughput = ledger.traces.len() as f64;
        let externalization_rate = if total_steps > 0 {
            hitl_steps as f64 / total_steps as f64
        } else {
            0.0
        };

        // Alignment cost in van der Aalst terms is the count of non-fitting steps.
        let alignment_cost = (1.0 - report.fitness) * total_steps as f64;

        Self {
            case_throughput,
            fitness: report.fitness,
            precision: report.precision,
            alignment_cost,
            externalization_rate,
            drift_epoch: latest_drift_epoch,
        }
    }
}

impl SubmissionPacket {
    /// Reassemble the 64-byte signature from `(signature_lo, signature_hi)`.
    #[must_use]
    pub fn signature_bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&self.signature_lo);
        out[32..].copy_from_slice(&self.signature_hi);
        out
    }
}

impl SubmissionPacket {
    /// Sign `(chain_head, bundle_hash)` with `key` to produce a packet.
    #[must_use]
    pub fn sign(chain_head: [u8; 32], bundle_hash: [u8; 32], key: &SigningKey) -> Self {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&chain_head);
        buf[32..].copy_from_slice(&bundle_hash);
        let sig: Signature = key.sign(&buf);
        let sig_bytes = sig.to_bytes();
        let mut signature_lo = [0u8; 32];
        let mut signature_hi = [0u8; 32];
        signature_lo.copy_from_slice(&sig_bytes[..32]);
        signature_hi.copy_from_slice(&sig_bytes[32..]);
        Self {
            chain_head,
            bundle_hash,
            signature_lo,
            signature_hi,
            pubkey: key.verifying_key().to_bytes(),
        }
    }

    /// Verify the embedded signature against `pubkey`.
    ///
    /// # Errors
    ///
    /// Returns `Err(())` when the public key is malformed or the signature fails.
    #[allow(clippy::result_unit_err)]
    pub fn verify(&self) -> Result<(), ()> {
        let vk = VerifyingKey::from_bytes(&self.pubkey).map_err(|_| ())?;
        let sig = Signature::from_bytes(&self.signature_bytes());
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&self.chain_head);
        buf[32..].copy_from_slice(&self.bundle_hash);
        vk.verify(&buf, &sig).map_err(|_| ())
    }

    /// Render the packet as RFC6962-compatible JSON bytes.
    #[must_use]
    pub fn to_rfc6962_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("SubmissionPacket is always serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::{rngs::OsRng, RngCore};

    fn key() -> SigningKey {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        SigningKey::from_bytes(&secret)
    }

    #[test]
    fn signature_roundtrips() {
        let k = key();
        let p = SubmissionPacket::sign([1u8; 32], [2u8; 32], &k);
        p.verify().expect("genuine signature must verify");
    }

    #[test]
    fn submission_packet_signature_tamper_fails_verify() {
        let k = key();
        let mut p = SubmissionPacket::sign([1u8; 32], [2u8; 32], &k);
        p.signature_lo[0] ^= 0x01;
        assert!(p.verify().is_err(), "tampered signature must fail verify");
    }

    #[test]
    fn scorecard_generation_metrics() {
        let mut ledger = EvidenceLedger::new();
        let mut trace = crate::powl64::Powl64::new();

        // Add a step with HITL projection
        trace.extend(crate::powl64::Powl64RouteCell {
            projection_target: ProjectionTarget::Hitl,
            chain_head: 1234,
            ..Default::default()
        });
        // Add a normal step
        trace.extend(crate::powl64::Powl64RouteCell {
            projection_target: ProjectionTarget::NoOp,
            chain_head: 5678,
            ..Default::default()
        });

        ledger.record(trace);

        let report = ConformanceReport {
            fitness: 0.5,
            precision: 0.8,
            false_closures: vec![],
            generalization: 0.0,
            simplicity: 0.0,
        };

        let scorecard = VanDerAalstScorecard::generate(&ledger, &report);

        assert_eq!(scorecard.case_throughput, 1.0);
        assert_eq!(scorecard.fitness, 0.5);
        assert_eq!(scorecard.precision, 0.8);
        assert_eq!(scorecard.externalization_rate, 0.5); // 1 Hitl out of 2 steps
        assert_eq!(scorecard.alignment_cost, 1.0); // (1.0 - 0.5) * 2 steps = 1.0
        assert_eq!(scorecard.drift_epoch, 5678); // Latest chain_head with fitness < 1.0
    }
}
