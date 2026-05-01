//! PROV-O receipt with BLAKE3 proof-of-origin hashing.

use crate::graph::GraphIri;
use chrono::{DateTime, Utc};

/// PROV receipt with cryptographic proof and deterministic hash.
#[derive(Clone, Debug)]
pub struct Receipt {
    /// IRI of the prov:Activity that generated the outcome.
    pub activity_iri: GraphIri,

    /// BLAKE3 hash of the activity's inputs, rules, and outputs.
    /// Hex-encoded, 64 characters (256 bits).
    pub hash: String,

    /// Timestamp when the receipt was generated.
    pub timestamp: DateTime<Utc>,
}

impl Receipt {
    /// Create a new receipt.
    pub fn new(activity_iri: GraphIri, hash: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            activity_iri,
            hash,
            timestamp,
        }
    }

    /// Generate a BLAKE3 hash from input data.
    pub fn blake3_hex(data: &[u8]) -> String {
        let hash = blake3::hash(data);
        hash.to_hex().to_string()
    }

    /// Parse this receipt's hex hash back into a typed `blake3::Hash`.
    ///
    /// # Errors
    ///
    /// Returns `Err(message)` if `self.hash` is not a valid 64-character
    /// BLAKE3 hex digest.
    pub fn chain_hash(&self) -> Result<blake3::Hash, String> {
        blake3::Hash::from_hex(&self.hash)
            .map_err(|e| format!("Receipt::chain_hash: invalid hex '{}': {}", self.hash, e))
    }

    /// Derive a deterministic `urn:blake3:{hex}` URN from canonical receipt material.
    ///
    /// The canonical material layout is caller's responsibility. This function
    /// only hashes the bytes via BLAKE3 and prefixes with `urn:blake3:`.
    ///
    /// Use [`Receipt::canonical_material`] to build the canonical byte layout:
    ///
    /// ```text
    /// hook_id_bytes || 0x00 || plan_node_le_u16_bytes || 0x00 || delta_receipt_bytes || 0x00 ||
    /// field_id_bytes || 0x00 || prior_chain_hash_bytes_or_32_zero_bytes || 0x00 || polarity_u8
    /// ```
    pub fn derive_urn(material: &[u8]) -> String {
        format!("urn:blake3:{}", blake3::hash(material).to_hex())
    }

    /// Build canonical receipt material bytes per the documented layout.
    ///
    /// Layout (NUL-separated, no length prefixes):
    ///
    /// ```text
    /// hook_id_bytes || 0x00 || plan_node_le_u16_bytes || 0x00 || delta_receipt_bytes || 0x00 ||
    /// field_id_bytes || 0x00 || prior_chain_hash_bytes_or_32_zero_bytes || 0x00 || polarity_u8
    /// ```
    ///
    /// `prior_chain = None` serializes as 32 zero bytes — the canonical
    /// sentinel for a chain origin. `prior_chain = Some([0u8; 32])` produces
    /// the same canonical material (and therefore the same derived URN).
    pub fn canonical_material(
        hook_id: &str,
        plan_node: u16,
        delta_receipt: &[u8],
        field_id: &str,
        prior_chain: Option<blake3::Hash>,
        polarity: u8,
    ) -> Vec<u8> {
        let prior_bytes: [u8; 32] = match prior_chain {
            Some(h) => *h.as_bytes(),
            None => [0u8; 32],
        };
        let plan_node_bytes = plan_node.to_le_bytes();

        let mut material = Vec::with_capacity(
            hook_id.len()
                + 1
                + plan_node_bytes.len()
                + 1
                + delta_receipt.len()
                + 1
                + field_id.len()
                + 1
                + prior_bytes.len()
                + 1
                + 1,
        );
        material.extend_from_slice(hook_id.as_bytes());
        material.push(0x00);
        material.extend_from_slice(&plan_node_bytes);
        material.push(0x00);
        material.extend_from_slice(delta_receipt);
        material.push(0x00);
        material.extend_from_slice(field_id.as_bytes());
        material.push(0x00);
        material.extend_from_slice(&prior_bytes);
        material.push(0x00);
        material.push(polarity);
        material
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_urn_is_deterministic() {
        let material = b"deterministic-material";
        let u1 = Receipt::derive_urn(material);
        let u2 = Receipt::derive_urn(material);
        assert_eq!(u1, u2, "same input must yield identical URN");
    }

    #[test]
    fn derive_urn_format_is_urn_blake3_with_64_hex() {
        let urn = Receipt::derive_urn(b"any bytes");
        assert!(
            urn.starts_with("urn:blake3:"),
            "URN must start with 'urn:blake3:', got {}",
            urn
        );
        let suffix = &urn["urn:blake3:".len()..];
        assert_eq!(
            suffix.len(),
            64,
            "BLAKE3 hex suffix must be 64 chars, got {} chars",
            suffix.len()
        );
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit()),
            "suffix must be ascii hex digits: {}",
            suffix
        );
    }

    #[test]
    fn prior_chain_none_matches_some_zeros() {
        // None must serialize exactly the same as Some([0u8; 32]).
        let zeros = blake3::Hash::from_bytes([0u8; 32]);
        let m_none = Receipt::canonical_material("hook", 0, b"delta", "field", None, 1);
        let m_some_zero = Receipt::canonical_material("hook", 0, b"delta", "field", Some(zeros), 1);
        assert_eq!(m_none, m_some_zero, "None must equal Some([0u8; 32])");
        assert_eq!(
            Receipt::derive_urn(&m_none),
            Receipt::derive_urn(&m_some_zero),
            "URNs must be identical"
        );
    }

    #[test]
    fn polarity_changes_urn() {
        let m1 = Receipt::canonical_material("hook", 0, b"delta", "field", None, 0);
        let m2 = Receipt::canonical_material("hook", 0, b"delta", "field", None, 1);
        assert_ne!(m1, m2, "polarity must affect canonical material");
        assert_ne!(
            Receipt::derive_urn(&m1),
            Receipt::derive_urn(&m2),
            "polarity must affect derived URN"
        );
    }

    #[test]
    fn distinct_inputs_distinct_urns() {
        let u1 = Receipt::derive_urn(b"a");
        let u2 = Receipt::derive_urn(b"b");
        assert_ne!(u1, u2);
    }

    #[test]
    fn canonical_material_layout_is_nul_separated() {
        // Spot-check that components are present in the right order
        // separated by NUL bytes (no length prefixes).
        let mat = Receipt::canonical_material("h", 0x0102, b"d", "f", None, 1);
        // hook_id 'h' || 0x00 || 0x02 0x01 (le u16) || 0x00 || 'd' || 0x00 ||
        // 'f' || 0x00 || 32 zero bytes || 0x00 || 0x01
        assert_eq!(mat[0], b'h');
        assert_eq!(mat[1], 0x00);
        assert_eq!(mat[2], 0x02);
        assert_eq!(mat[3], 0x01);
        assert_eq!(mat[4], 0x00);
        assert_eq!(mat[5], b'd');
        assert_eq!(mat[6], 0x00);
        assert_eq!(mat[7], b'f');
        assert_eq!(mat[8], 0x00);
        // 32 zero bytes for None prior_chain
        for (i, &byte) in mat.iter().enumerate().skip(9).take(32) {
            assert_eq!(byte, 0x00, "expected zero byte at offset {}", i);
        }
        assert_eq!(mat[9 + 32], 0x00);
        assert_eq!(mat[9 + 32 + 1], 0x01);
        assert_eq!(mat.len(), 9 + 32 + 2);
    }
}
