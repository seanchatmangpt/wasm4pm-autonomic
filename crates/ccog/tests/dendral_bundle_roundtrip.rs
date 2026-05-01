//! Phase 11 — DENDRAL bundle roundtrip + tampering tests.
//!
//! Boundary-detector tests are LOAD-BEARING. Do not delete them under
//! refactor — they encode the only externally-visible guardrails on the
//! transparency surface.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use ccog::compiled::CompiledFieldSnapshot;
use ccog::export::bundle::{BundleError, ProofBundle};
use ccog::export::jsonld::{canonical_bytes, receipt_to_jsonld, trace_to_jsonld};
use ccog::export::ontology::{audit_iris, NonPublicOntology};
use ccog::export::replay::{verify_bundle, verify_bundle_bytes};
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::receipt::Receipt;
use ccog::runtime::ClosedFieldContext;
use ccog::trace::{trace_default_builtins, BenchmarkTier};

// =============================================================================
// FIXTURES
// =============================================================================

fn fixture_field() -> FieldContext {
    let mut field = FieldContext::new("phase11-fixture");
    field
        .load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"alpha\" .\n",
        )
        .expect("load");
    field
}

fn fixture_receipt() -> Receipt {
    let material = Receipt::canonical_material("phrase_binding", 1, b"delta", "phase11", None, 1);
    let urn = Receipt::derive_urn(&material);
    let iri = ccog::graph::GraphIri::from_iri(&urn).expect("urn:blake3 IRI");
    let hash = Receipt::blake3_hex(&material);
    Receipt::new(iri, hash, chrono::Utc::now())
}

fn fixture_path_3_entries() -> Vec<u8> {
    let h1 = blake3::hash(b"step-1");
    let h2 = blake3::hash(b"step-2");
    let h3 = blake3::hash(b"step-3");
    let mut bytes = Vec::with_capacity(96);
    bytes.extend_from_slice(h1.as_bytes());
    bytes.extend_from_slice(h2.as_bytes());
    bytes.extend_from_slice(h3.as_bytes());
    bytes
}

fn build_genuine_bundle() -> ProofBundle {
    let field = fixture_field();
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    let trace = trace_default_builtins(&context);
    let trace_bytes = canonical_bytes(&trace_to_jsonld(&trace));
    let receipt = fixture_receipt();
    let receipt_bytes = canonical_bytes(&receipt_to_jsonld(&receipt));
    let path = fixture_path_3_entries();

    let trace_v: serde_json::Value = serde_json::from_slice(&trace_bytes).unwrap();
    let receipt_v: serde_json::Value = serde_json::from_slice(&receipt_bytes).unwrap();
    let mut refs: Vec<String> = Vec::new();
    refs.extend(audit_iris(&trace_v, &[]).expect("trace iris must be public"));
    refs.extend(audit_iris(&receipt_v, &[]).expect("receipt iris must be public"));

    ProofBundle::build(
        trace_bytes,
        receipt_bytes,
        path,
        refs,
        BenchmarkTier::ConformanceReplay,
    )
}

/// Re-serialize an entry map into a tampered `.tar.zst` blob WITHOUT
/// updating `manifest.json` — used by tamper-detection tests to inject
/// modifications behind the manifest's back.
fn tar_zst_from_entries(entries: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut tar_bytes: Vec<u8> = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        builder.mode(tar::HeaderMode::Deterministic);
        for (name, data) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_uid(0);
            h.set_gid(0);
            h.set_mtime(0);
            h.set_entry_type(tar::EntryType::Regular);
            h.set_cksum();
            builder
                .append_data(&mut h, name, Cursor::new(data))
                .expect("append");
        }
        builder.finish().expect("finish");
    }
    let mut compressed: Vec<u8> = Vec::new();
    {
        let mut enc = zstd::Encoder::new(&mut compressed, 19).expect("encoder");
        enc.write_all(&tar_bytes).expect("write");
        enc.finish().expect("finish");
    }
    compressed
}

fn read_back_entries(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut decoded: Vec<u8> = Vec::new();
    let mut dec = zstd::Decoder::new(bytes).expect("zstd");
    dec.read_to_end(&mut decoded).expect("read");
    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut a = tar::Archive::new(Cursor::new(&decoded));
    for e in a.entries().expect("entries") {
        let mut e = e.expect("entry");
        let p = e.path().expect("path").to_string_lossy().to_string();
        let mut buf = Vec::new();
        e.read_to_end(&mut buf).expect("read");
        entries.insert(p, buf);
    }
    entries
}

// =============================================================================
// POSITIVE TESTS
// =============================================================================

#[test]
fn jsonld_trace_roundtrip_stable_bytes() {
    let field = fixture_field();
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    let trace = trace_default_builtins(&context);
    let b1 = canonical_bytes(&trace_to_jsonld(&trace));
    let b2 = canonical_bytes(&trace_to_jsonld(&trace));
    assert_eq!(b1, b2, "trace JSON-LD must be byte-stable");
}

#[test]
fn jsonld_context_only_public_iris() {
    let field = fixture_field();
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let context = ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
    };
    let trace = trace_default_builtins(&context);
    let v = trace_to_jsonld(&trace);
    audit_iris(&v, &[]).expect("trace JSON-LD must contain only public IRIs");

    let receipt = fixture_receipt();
    let rv = receipt_to_jsonld(&receipt);
    audit_iris(&rv, &[]).expect("receipt JSON-LD must contain only public IRIs");
}

#[test]
fn bundle_write_read_roundtrip() {
    let bundle = build_genuine_bundle();
    let bytes = bundle.write().expect("write");
    let bundle2 = ProofBundle::read(&bytes).expect("read");
    assert_eq!(bundle.entries, bundle2.entries);
}

#[test]
fn bundle_deterministic_bytes() {
    let bundle = build_genuine_bundle();
    let x = bundle.write().expect("write 1");
    let y = bundle.write().expect("write 2");
    assert_eq!(x, y, "bundle bytes must be deterministic");
}

#[test]
fn replay_verifier_accepts_genuine_bundle() {
    let bundle = build_genuine_bundle();
    let v = verify_bundle(&bundle, &[], None).expect("verify");
    assert!(
        v.manifest_intact,
        "manifest must be intact for genuine bundle"
    );
    assert!(
        v.ontology_intact,
        "ontology must be intact for genuine bundle"
    );
    assert!(
        v.chain_match,
        "chain must be self-consistent for genuine bundle"
    );
    assert!(
        v.decision_match,
        "decision invariant must hold for genuine bundle"
    );
    assert!(v.all_intact());
}

#[test]
fn powl64_path_matches_chain_head() {
    // Chain head = the trailing 32-byte chunk of powl64-path.bin. This
    // mirrors how dendral's `path` subcommand checks consistency.
    let path = fixture_path_3_entries();
    assert_eq!(path.len(), 96);
    let tail: [u8; 32] = path[64..].try_into().expect("32 bytes");
    let recovered = blake3::hash(b"step-3");
    assert_eq!(tail, *recovered.as_bytes());
}

// =============================================================================
// TAMPERING TESTS — MANDATORY BOUNDARY DETECTORS, DO NOT DELETE
// =============================================================================

#[test]
fn tamper_trace_jsonld_fails_manifest() {
    let bundle = build_genuine_bundle();
    let bytes = bundle.write().expect("write");
    let mut entries = read_back_entries(&bytes);
    entries.insert("trace.jsonld".into(), b"{\"tampered\":true}".to_vec());
    // Do NOT update manifest.json.
    let tampered = tar_zst_from_entries(&entries);
    let err = ProofBundle::read(&tampered).expect_err("must reject tampered trace");
    match err {
        BundleError::ManifestMismatch { name, .. } => assert_eq!(name, "trace.jsonld"),
        other => panic!("expected ManifestMismatch, got {:?}", other),
    }
}

#[test]
fn tamper_receipt_polarity_fails_replay() {
    // Build a receipt with polarity 1; tamper to polarity 0; the URN
    // changes; replay/verify rejects via manifest mismatch.
    let bundle = build_genuine_bundle();
    let bytes = bundle.write().expect("write");
    let mut entries = read_back_entries(&bytes);

    // Replace the receipt JSON-LD with a polarity-0-derived URN.
    let mat0 = Receipt::canonical_material("phrase_binding", 1, b"delta", "phase11", None, 0);
    let urn0 = Receipt::derive_urn(&mat0);
    let iri0 = ccog::graph::GraphIri::from_iri(&urn0).expect("urn");
    let receipt0 = Receipt::new(iri0, Receipt::blake3_hex(&mat0), chrono::Utc::now());
    let new_bytes = canonical_bytes(&receipt_to_jsonld(&receipt0));
    entries.insert("receipt.jsonld".into(), new_bytes);

    let tampered = tar_zst_from_entries(&entries);
    let v = verify_bundle_bytes(&tampered, &[], None).expect("verify call");
    assert!(
        !v.manifest_intact,
        "polarity tamper must surface as manifest mismatch"
    );
}

#[test]
fn tamper_powl64_path_breaks_chain_match() {
    // Replace the path with a non-32-aligned blob. After re-tar with manifest
    // updated for the new path, manifest matches but chain self-consistency
    // fails inside replay::verify_bundle.
    let bundle = build_genuine_bundle();
    let mut tampered_entries = bundle.entries.clone();
    tampered_entries.insert("powl64-path.bin".into(), vec![0xAA; 17]); // odd length
                                                                       // Recompute manifest so the manifest layer accepts the entries.
    let mut map = serde_json::Map::new();
    for (k, v) in &tampered_entries {
        if k == "manifest.json" {
            continue;
        }
        map.insert(
            k.clone(),
            serde_json::Value::String(blake3::hash(v).to_hex().to_string()),
        );
    }
    tampered_entries.insert(
        "manifest.json".into(),
        serde_json::to_vec(&serde_json::Value::Object(map)).unwrap(),
    );
    let tampered_bundle = ProofBundle {
        entries: tampered_entries,
    };
    let v = verify_bundle(&tampered_bundle, &[], None).expect("verify call");
    assert!(v.manifest_intact);
    assert!(
        !v.chain_match,
        "non-32-aligned path must fail chain self-consistency"
    );
}

#[test]
fn inject_unknown_iri_fails_ontology_audit() {
    let v = serde_json::json!({"@type": "http://attacker.example/Activity"});
    let err = audit_iris(&v, &[]).expect_err("attacker IRI must be rejected");
    let NonPublicOntology(iri) = err;
    assert!(iri.contains("attacker.example"));
}

#[test]
fn truncate_chain_one_node_fails_chain_match() {
    // Verify with an expected chain head equal to the original tail, but
    // replace the path with a TRUNCATED version (last node removed). The
    // declared expected_chain_head no longer matches the new tail; check
    // surfaces via chain_match=false.
    let bundle = build_genuine_bundle();
    let mut entries = bundle.entries.clone();
    let original_path = entries.get("powl64-path.bin").cloned().unwrap();
    let original_tail: [u8; 32] = original_path[64..].try_into().unwrap();

    // Truncate to the first 64 bytes (drop step-3).
    entries.insert("powl64-path.bin".into(), original_path[..64].to_vec());
    // Update manifest so the manifest layer accepts.
    let mut map = serde_json::Map::new();
    for (k, v) in &entries {
        if k == "manifest.json" {
            continue;
        }
        map.insert(
            k.clone(),
            serde_json::Value::String(blake3::hash(v).to_hex().to_string()),
        );
    }
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec(&serde_json::Value::Object(map)).unwrap(),
    );
    let tampered = ProofBundle { entries };
    let v = verify_bundle(&tampered, &[], Some(original_tail)).expect("verify");
    assert!(v.manifest_intact);
    assert!(
        !v.chain_match,
        "truncated chain must not match the originally-declared head"
    );
}

#[test]
fn swap_two_path_hashes_fails_chain_match() {
    // Swap two adjacent 32-byte hashes; with `expected_chain_head`
    // pointing at the original tail, `chain_match` must be false
    // (tail differs after swap if the swapped pair includes the tail).
    let bundle = build_genuine_bundle();
    let mut entries = bundle.entries.clone();
    let mut path = entries.get("powl64-path.bin").cloned().unwrap();
    let original_tail: [u8; 32] = path[64..].try_into().unwrap();

    // Swap entries [1] and [2] (so the tail becomes step-2's hash).
    let h1: [u8; 32] = path[..32].try_into().unwrap();
    let h2: [u8; 32] = path[32..64].try_into().unwrap();
    let h3: [u8; 32] = path[64..96].try_into().unwrap();
    let mut swapped = Vec::with_capacity(96);
    swapped.extend_from_slice(&h1);
    swapped.extend_from_slice(&h3);
    swapped.extend_from_slice(&h2);
    path = swapped;

    entries.insert("powl64-path.bin".into(), path);
    let mut map = serde_json::Map::new();
    for (k, v) in &entries {
        if k == "manifest.json" {
            continue;
        }
        map.insert(
            k.clone(),
            serde_json::Value::String(blake3::hash(v).to_hex().to_string()),
        );
    }
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec(&serde_json::Value::Object(map)).unwrap(),
    );
    let tampered = ProofBundle { entries };
    let v = verify_bundle(&tampered, &[], Some(original_tail)).expect("verify");
    assert!(v.manifest_intact);
    assert!(
        !v.chain_match,
        "swapping two path hashes must change the tail, breaking chain_match"
    );
}

#[cfg(feature = "transparency")]
#[test]
fn submission_packet_signature_tamper_fails_verify() {
    use ccog::export::transparency::SubmissionPacket;
    use ed25519_dalek::SigningKey;
    use rand::{rngs::OsRng, RngCore};

    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let key = SigningKey::from_bytes(&secret);
    let mut pkt = SubmissionPacket::sign([7u8; 32], [9u8; 32], &key);
    pkt.signature_lo[0] ^= 0xFF;
    assert!(
        pkt.verify().is_err(),
        "tampered Ed25519 signature must fail verification"
    );
}

#[test]
fn bundle_with_private_namespace_rejected() {
    // Build a JSON-LD object with a private `urn:ccog:internal:` IRI; audit
    // must reject regardless of bundle context.
    let private = serde_json::json!({
        "@id": "urn:ccog:internal:secret-key",
        "@type": "http://www.w3.org/ns/prov#Activity",
    });
    let err = audit_iris(&private, &[]).expect_err("private namespace must be rejected");
    let NonPublicOntology(iri) = err;
    assert!(
        iri.starts_with("urn:ccog:internal:"),
        "expected private namespace IRI to be flagged, got: {}",
        iri
    );
}
