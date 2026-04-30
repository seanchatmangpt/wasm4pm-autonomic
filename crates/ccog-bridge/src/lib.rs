#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! `ccog-bridge` — translation layer between [`ccog`] and the parent
//! [`dteam`] crate.
//!
//! Phase 8.4 splits the ccog → dteam translation into its own workspace
//! member so that `ccog` itself stays free of any path dep on `dteam`
//! (the dep direction must point library → engine, never engine → library).
//! This crate is the single allowed place where both crates' types are
//! visible at once.
//!
//! # Surface
//!
//! - [`ontology_kbitset_to_present_mask`] projects a dteam `KBitSet<16>`
//!   ontology bitmask down to the 64-bit `present_mask` shape ccog
//!   consumes.
//! - [`trace_to_runtime_response`] converts a `ccog::CcogTrace` into a
//!   bridge-local `Response` summary suitable for handing to a
//!   dteam-side runtime.
//! - [`receipt_to_runtime_evidence`] flattens a `ccog::Receipt` into
//!   the bridge-local `Evidence` shape.

use ccog::{trace::CcogTrace, Receipt};
use dteam::utils::dense_kernel::KBitSet;

/// Translation entry mapping a dteam `KBitSet<16>` ontology bit (slot
/// `0..1024`) to the ccog 64-bit predicate-bit it should set on the
/// present mask. Out-of-range targets are silently dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KBitMap {
    /// Source bit index inside the dteam ontology bitset.
    pub kbit: usize,
    /// Destination bit index inside the ccog 64-bit present mask.
    pub predicate_bit: u32,
}

/// Default identity translation table — kbit `i` maps to predicate bit
/// `i` for `i < 64`. Callers wanting a non-identity mapping should
/// pass their own table to [`ontology_kbitset_to_present_mask_with`].
pub const DEFAULT_KBIT_MAP: &[KBitMap] = &[
    KBitMap { kbit: 0, predicate_bit: 0 },
    KBitMap { kbit: 1, predicate_bit: 1 },
    KBitMap { kbit: 2, predicate_bit: 2 },
    KBitMap { kbit: 3, predicate_bit: 3 },
    KBitMap { kbit: 4, predicate_bit: 4 },
    KBitMap { kbit: 5, predicate_bit: 5 },
    KBitMap { kbit: 6, predicate_bit: 6 },
    KBitMap { kbit: 7, predicate_bit: 7 },
];

/// Project a dteam `KBitSet<16>` (1024 logical bits across 16 u64 words)
/// onto a 64-bit ccog present mask, using the default identity table.
#[must_use]
pub fn ontology_kbitset_to_present_mask(kb: &KBitSet<16>) -> u64 {
    ontology_kbitset_to_present_mask_with(kb, DEFAULT_KBIT_MAP)
}

/// Project with a caller-provided translation table.
#[must_use]
pub fn ontology_kbitset_to_present_mask_with(kb: &KBitSet<16>, table: &[KBitMap]) -> u64 {
    let mut m = 0u64;
    for entry in table {
        if entry.predicate_bit >= 64 {
            continue;
        }
        if kb.contains(entry.kbit) {
            m |= 1u64 << entry.predicate_bit;
        }
    }
    m
}

/// Lightweight runtime response summary.
///
/// Bridge-local — defined here rather than under `dteam::runtime` to avoid
/// coupling the dteam runtime API to ccog's trace shape. A future move into
/// dteam proper would only widen this struct.
#[derive(Clone, Debug, Default)]
pub struct Response {
    /// Bitmask of canonical predicates present at the time the trace was taken.
    pub present_mask: u64,
    /// Number of slots whose `trigger_fired && check_passed`.
    pub fired_count: u32,
    /// Number of slots that were skipped for any reason.
    pub skipped_count: u32,
    /// Total number of slots inspected.
    pub total_slots: u32,
    /// Pack posture observed for the trace.
    pub posture: ccog::PackPosture,
}

/// Bridge-local evidence summary derived from a `ccog::Receipt`.
#[derive(Clone, Debug)]
pub struct Evidence {
    /// `urn:blake3:` IRI of the prov:Activity that generated the receipt.
    pub activity_iri: String,
    /// 32-byte BLAKE3 chain hash, hex-encoded.
    pub chain_hash_hex: String,
    /// Receipt timestamp expressed as RFC-3339 string.
    pub timestamp_rfc3339: String,
}

/// Convert a `CcogTrace` into a bridge `Response`.
#[must_use]
pub fn trace_to_runtime_response(trace: &CcogTrace) -> Response {
    Response {
        present_mask: trace.present_mask,
        fired_count: trace.fired_count() as u32,
        skipped_count: trace.skipped_count() as u32,
        total_slots: trace.nodes.len() as u32,
        posture: trace.posture,
    }
}

/// Convert a `ccog::Receipt` into a bridge `Evidence`.
#[must_use]
pub fn receipt_to_runtime_evidence(r: &Receipt) -> Evidence {
    Evidence {
        activity_iri: r.activity_iri.clone(),
        chain_hash_hex: r.hash.clone(),
        timestamp_rfc3339: r.timestamp.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kbitset_roundtrip_preserves_predicates() {
        let mut kb: KBitSet<16> = KBitSet::default();
        kb.set(0).unwrap();
        kb.set(3).unwrap();
        kb.set(7).unwrap();
        let m = ontology_kbitset_to_present_mask(&kb);
        assert_eq!(m, (1u64 << 0) | (1u64 << 3) | (1u64 << 7));
    }

    #[test]
    fn kbitset_oob_predicate_bit_is_dropped() {
        let mut kb: KBitSet<16> = KBitSet::default();
        kb.set(0).unwrap();
        let oob_table = [KBitMap { kbit: 0, predicate_bit: 999 }];
        let m = ontology_kbitset_to_present_mask_with(&kb, &oob_table);
        assert_eq!(m, 0);
    }

    #[test]
    fn trace_to_runtime_response_preserves_counts() {
        let mut trace = CcogTrace::default();
        trace.present_mask = 0b1011;
        // Two fired, one skipped.
        trace.nodes.push(ccog::trace::BarkNodeTrace {
            slot_idx: 0,
            hook_id: "a",
            require_mask: 0,
            predecessor_mask: 0,
            trigger_fired: true,
            check_passed: true,
            act_emitted_triples: 0,
            receipt_urn: None,
            skip_reason: None,
            skip: None,
        });
        trace.nodes.push(ccog::trace::BarkNodeTrace {
            slot_idx: 1,
            hook_id: "b",
            require_mask: 0,
            predecessor_mask: 0,
            trigger_fired: true,
            check_passed: true,
            act_emitted_triples: 0,
            receipt_urn: None,
            skip_reason: None,
            skip: None,
        });
        trace.nodes.push(ccog::trace::BarkNodeTrace {
            slot_idx: 2,
            hook_id: "c",
            require_mask: 0,
            predecessor_mask: 0,
            trigger_fired: false,
            check_passed: false,
            act_emitted_triples: 0,
            receipt_urn: None,
            skip_reason: Some("x"),
            skip: Some(ccog::trace::BarkSkipReason::RequireMaskUnsatisfied),
        });
        let r = trace_to_runtime_response(&trace);
        assert_eq!(r.present_mask, 0b1011);
        assert_eq!(r.fired_count, 2);
        assert_eq!(r.skipped_count, 1);
        assert_eq!(r.total_slots, 3);
    }

    #[test]
    fn receipt_to_runtime_evidence_flattens_fields() {
        let r = Receipt::new(
            "urn:blake3:deadbeef".to_string(),
            "00".repeat(32),
            chrono::Utc::now(),
        );
        let e = receipt_to_runtime_evidence(&r);
        assert_eq!(e.activity_iri, "urn:blake3:deadbeef");
        assert_eq!(e.chain_hash_hex.len(), 64);
        assert!(!e.timestamp_rfc3339.is_empty());
    }
}
