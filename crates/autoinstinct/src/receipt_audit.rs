//! Phase 5 — Receipt-sensitivity surface.
//!
//! Mirrors `ccog::receipt::canonical_material` semantic identity rules at
//! the AutoInstinct layer: same material ⇒ same URN; any change to hook id,
//! plan node, delta bytes, field id, prior chain, or polarity ⇒ different
//! URN. Wall-clock time NEVER influences identity.

use serde::{Deserialize, Serialize};

/// Inputs to receipt material derivation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptMaterial {
    /// Hook identifier.
    pub hook_id: String,
    /// Plan-node index.
    pub plan_node: u16,
    /// Delta bytes (the materialized writeback).
    pub delta: Vec<u8>,
    /// Field identifier.
    pub field_id: String,
    /// Prior chain hash, or None for genesis.
    pub prior_chain: Option<[u8; 32]>,
    /// Polarity bit (0 = denial, 1 = admission).
    pub polarity: u8,
}

/// Compute the canonical bytes that get hashed into a receipt URN.
/// Matches `ccog::receipt::canonical_material` byte-for-byte.
#[must_use]
pub fn canonical_bytes(m: &ReceiptMaterial) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        m.hook_id.len() + 2 + m.delta.len() + m.field_id.len() + 32 + 6,
    );
    out.extend_from_slice(m.hook_id.as_bytes());
    out.push(0);
    out.extend_from_slice(&m.plan_node.to_le_bytes());
    out.push(0);
    out.extend_from_slice(&m.delta);
    out.push(0);
    out.extend_from_slice(m.field_id.as_bytes());
    out.push(0);
    if let Some(p) = m.prior_chain {
        out.extend_from_slice(&p);
    } else {
        out.extend_from_slice(&[0u8; 32]);
    }
    out.push(0);
    out.push(m.polarity);
    out
}

/// Derive `urn:blake3:` URN from the material.
#[must_use]
pub fn derive_urn(m: &ReceiptMaterial) -> String {
    let bytes = canonical_bytes(m);
    let h = blake3::hash(&bytes);
    format!("urn:blake3:{}", h.to_hex())
}

/// Audit verdict.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptAuditVerdict {
    /// True iff every required perturbation produced a different URN.
    pub all_axes_sensitive: bool,
    /// True iff identical inputs produced identical URNs.
    pub deterministic: bool,
}

/// Audit a baseline material across the six perturbation axes from the
/// AutoInstinct README receipt scenario.
#[must_use]
pub fn audit(base: &ReceiptMaterial) -> ReceiptAuditVerdict {
    let urn0 = derive_urn(base);
    let urn0_again = derive_urn(base);
    let deterministic = urn0 == urn0_again;

    let mut variants: Vec<ReceiptMaterial> = Vec::new();
    let mut a = base.clone();
    a.hook_id.push('!');
    variants.push(a);
    let mut b = base.clone();
    b.plan_node = b.plan_node.wrapping_add(1);
    variants.push(b);
    let mut c = base.clone();
    c.delta.push(0xff);
    variants.push(c);
    let mut d = base.clone();
    d.field_id.push('!');
    variants.push(d);
    let mut e = base.clone();
    e.prior_chain = match e.prior_chain {
        Some(_) => None,
        None => Some([0xab; 32]),
    };
    variants.push(e);
    let mut f = base.clone();
    f.polarity ^= 1;
    variants.push(f);

    let all_axes_sensitive = variants.iter().all(|v| derive_urn(v) != urn0);
    ReceiptAuditVerdict {
        all_axes_sensitive,
        deterministic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> ReceiptMaterial {
        ReceiptMaterial {
            hook_id: "h".into(),
            plan_node: 1,
            delta: b"d".to_vec(),
            field_id: "f".into(),
            prior_chain: None,
            polarity: 1,
        }
    }

    #[test]
    fn receipt_audit_admits_genuine_material() {
        let v = audit(&base());
        assert!(v.deterministic);
        assert!(v.all_axes_sensitive);
    }

    #[test]
    fn urn_starts_with_urn_blake3() {
        let urn = derive_urn(&base());
        assert!(urn.starts_with("urn:blake3:"));
    }

    #[test]
    fn polarity_flip_changes_urn() {
        let a = base();
        let mut b = base();
        b.polarity ^= 1;
        assert_ne!(derive_urn(&a), derive_urn(&b));
    }
}
