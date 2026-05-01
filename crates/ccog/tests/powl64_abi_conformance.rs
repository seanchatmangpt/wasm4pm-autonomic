//! POWL64 ABI conformance harness.

use ccog::abi::{powl64_from_postcard, powl64_to_jsonld, powl64_to_postcard};
use ccog::powl64::{Powl64, Powl64RouteCell};

#[test]
fn replay_preserves_path() {
    let mut p = Powl64::new();
    for i in 0..8 {
        p.extend(Powl64RouteCell {
            chain_head: i as u64,
            ..Default::default()
        });
    }

    let mut q = Powl64::new();
    for i in 0..8 {
        q.extend(Powl64RouteCell {
            chain_head: i as u64,
            ..Default::default()
        });
    }
    assert!(p.shape_match_v1_path(&q).is_ok());
}

#[test]
fn chain_len_equals_path_len() {
    let mut p = Powl64::new();
    for i in 0..5 {
        p.extend(Powl64RouteCell {
            chain_head: i as u64,
            ..Default::default()
        });
    }
    assert_eq!(p.chain_len(), p.path().len());
}

#[test]
fn powl64_postcard_roundtrip_preserves_path() {
    let mut p = Powl64::new();
    for i in 0..4 {
        p.extend(Powl64RouteCell {
            chain_head: i as u64,
            ..Default::default()
        });
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
    p.extend(Powl64RouteCell {
        chain_head: 1,
        ..Default::default()
    });
    p.extend(Powl64RouteCell {
        chain_head: 2,
        ..Default::default()
    });
    let v = powl64_to_jsonld(&p);
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("urn:blake3:"));
    assert!(s.contains("ccog:Powl64Path"));
}
