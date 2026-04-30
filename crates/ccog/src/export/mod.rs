//! Phase 11 — External Proof / Transparency.
//!
//! - [`jsonld`] — deterministic JSON-LD serializer for traces and receipts.
//! - [`bundle`] — `.tar.zst` proof bundle (sorted entries, mtime=0, zstd 19).
//! - [`ontology`] — public-ontology IRI audit (allowlist + `extra_allow`).
//! - [`replay`] — replay verifier (`ReplayVerdict`).
//! - [`transparency`] — Ed25519 submission packet (feature-gated).
//!
//! ccog stays library-only. The DENDRAL CLI lives in
//! `crates/ccog/src/bin/dendral.rs` and consumes these modules.

pub mod bundle;
pub mod jsonld;
pub mod ontology;
pub mod replay;
#[cfg(feature = "transparency")]
pub mod transparency;
