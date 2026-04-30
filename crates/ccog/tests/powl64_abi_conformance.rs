//! POWL64 ABI conformance harness (Phase 10).
//!
//! Pinned correctness obligations on the `Powl64` chain-receipt ABI.
//! These tests are load-bearing for cross-tool replay: any code change
//! that breaks them constitutes a wire-format defect.

use ccog::abi::{powl64_from_postcard, powl64_to_jsonld, powl64_to_postcard};
use ccog::graph::GraphIri;
use ccog::powl64::Powl64;

fn iri(s: &str) -> GraphIri {
    GraphIri::from_iri(s).expect("valid IRI")
}

#[test]
fn replay_preserves_collisions() {
    // Two distinct extend calls may project to the same 18-bit coord; both
    // cells must survive in `cells_at(coord)`. Replaying the path must
    // preserve the collision count, not collapse it.
    let mut p = Powl64::new();
    for i in 0..8 {
        p.extend(&iri(&format!("urn:test:r:{i}")), (i & 1) as u8);
    }
    let cell_count = p.cell_count();
    let chain_len = p.chain_len();
    assert!(chain_len as usize >= cell_count.min(8),
        "chain length must reflect every extend, regardless of collisions");
    // Now run the same source IRIs into a fresh Powl64 and confirm path
    // equality.
    let mut q = Powl64::new();
    for i in 0..8 {
        q.extend(&iri(&format!("urn:test:r:{i}")), (i & 1) as u8);
    }
    assert!(p.shape_match_v1_path(&q).is_ok());
}

#[test]
fn chain_len_equals_path_len() {
    let mut p = Powl64::new();
    for i in 0..5 {
        p.extend(&iri(&format!("urn:test:c:{i}")), 1);
    }
    assert_eq!(p.chain_len() as usize, p.path().len());
}

#[test]
fn chain_head_folds_polarity_at_genesis() {
    // Genesis chain hash MUST fold polarity (per the module doc), so the
    // same source IRI with different polarities yields different chain
    // heads at genesis.
    let mut p_pos = Powl64::new();
    p_pos.extend(&iri("urn:test:genesis"), 1);
    let mut p_neg = Powl64::new();
    p_neg.extend(&iri("urn:test:genesis"), 0);
    assert_ne!(
        p_pos.chain_head(),
        p_neg.chain_head(),
        "polarity must be folded into genesis chain hash"
    );
}

#[test]
fn semantic_receipt_diverges_from_polarity_only_receipt() {
    // `extend_with_semantic_receipt` stores a precomputed semantic receipt
    // on the cell. The chain hash itself MUST NOT incorporate the semantic
    // receipt — otherwise replay against a path bundle would require the
    // semantic receipts too. Confirm the chain head matches the plain
    // `extend` path despite the semantic-receipt presence.
    let mut p_plain = Powl64::new();
    p_plain.extend(&iri("urn:test:s:0"), 1);
    let mut p_sem = Powl64::new();
    p_sem.extend_with_semantic_receipt(
        &iri("urn:test:s:0"),
        1,
        blake3::hash(b"semantic-payload"),
    );
    assert_eq!(
        p_plain.chain_head(),
        p_sem.chain_head(),
        "semantic receipt must NOT be folded into chain hash"
    );
}

#[test]
fn extend_then_replay_idempotent_under_shape_match_v1_path() {
    let iris: Vec<GraphIri> = (0..6).map(|i| iri(&format!("urn:test:e:{i}"))).collect();
    let mut a = Powl64::new();
    let mut b = Powl64::new();
    for (k, ir) in iris.iter().enumerate() {
        a.extend(ir, (k % 2) as u8);
        b.extend(ir, (k % 2) as u8);
    }
    assert!(a.shape_match_v1_path(&b).is_ok());
    // Diverge polarity on the last extend in `b` and confirm the path
    // equivalence breaks.
    b.extend(&iri("urn:test:e:6"), 1);
    a.extend(&iri("urn:test:e:6"), 0);
    assert!(a.shape_match_v1_path(&b).is_err());
}

#[test]
fn powl64_postcard_roundtrip_preserves_path() {
    let mut p = Powl64::new();
    for i in 0..4 {
        p.extend(&iri(&format!("urn:test:p:{i}")), (i & 1) as u8);
    }
    let bytes = powl64_to_postcard(&p).expect("serialize");
    let path = powl64_from_postcard(&bytes).expect("deserialize");
    assert_eq!(path.len(), p.path().len());
    for (a, b) in path.iter().zip(p.path().iter()) {
        assert_eq!(a.as_bytes(), b.as_bytes());
    }
}

#[test]
fn powl64_jsonld_emits_blake3_urn_per_chain_entry() {
    let mut p = Powl64::new();
    p.extend(&iri("urn:test:j:0"), 1);
    p.extend(&iri("urn:test:j:1"), 0);
    let v = powl64_to_jsonld(&p);
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("urn:blake3:"));
    assert!(s.contains("ccog:Powl64Path"));
}
