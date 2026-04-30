//! POWL ABI serialization (Phase 10) — JSON-LD / postcard / debug Display.
//!
//! These functions are **offline-only**: they are never on the `decide()` /
//! `materialize()` / `seal()` hot path. They serve audit, replay, and human
//! review.
//!
//! - [`jsonld`] — JSON-LD with PROV-O `@context` and deterministic key order
//!   (BTreeMap-based). Used by external auditors.
//! - [`binary`] — postcard byte serialization (canonical, allocation-light).
//!   Used for runtime IPC and on-disk plan caches. Postcard is preferred over
//!   bincode (non-canonical) and rkyv (would require `unsafe`).
//! - [`debug`] — indented `Display` impls for human review of plan trees.

pub mod binary;
pub mod debug;
pub mod jsonld;

pub use binary::{
    powl64_from_postcard, powl64_to_postcard, powl8_from_postcard, powl8_to_postcard,
};
pub use debug::{powl64_to_debug_string, powl8_to_debug_string};
pub use jsonld::{powl64_to_jsonld, powl8_to_jsonld};
