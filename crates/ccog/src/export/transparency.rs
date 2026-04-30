//! Transparency anchoring (Phase 11.6).
//!
//! Off by default. Enabled via the `transparency` Cargo feature, which
//! pulls in `ed25519-dalek` for signing. Packet shape is RFC6962-style;
//! ccog itself does **not** depend on `sigstore-rs`.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

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
}
