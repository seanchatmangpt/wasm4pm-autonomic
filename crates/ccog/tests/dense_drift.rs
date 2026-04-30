//! Drift detector for `dteam/src/utils/dense_kernel.rs`.
//!
//! `crates/ccog/src/utils/dense.rs` is a deliberate rewrite of the upstream
//! `dteam/src/utils/dense_kernel.rs` that drops `unsafe`, drops serde, and
//! pares the surface to ccog's hot path. We **keep** the copy (Phase 8.3
//! decision) — depending on `dteam` from a library crate would reverse the
//! dep direction and re-poison `cargo -p ccog` with the parent's unrelated
//! errors.
//!
//! This test BLAKE3-hashes the upstream file and asserts the hash matches a
//! pinned constant. Intentional sync requires explicit hash bump.

const PINNED_DENSE_KERNEL_HASH: &str =
    "3c54df72de4b8fe31b039bf3b0638147cf9f4c02b25f2432c67fa743620f44b0";

#[test]
fn dense_kernel_upstream_hash_pinned() {
    let upstream: &str = include_str!("../../../src/utils/dense_kernel.rs");
    let h = blake3::hash(upstream.as_bytes()).to_hex().to_string();
    assert_eq!(
        h, PINNED_DENSE_KERNEL_HASH,
        "dteam/src/utils/dense_kernel.rs has drifted. Audit the diff vs \
         crates/ccog/src/utils/dense.rs and bump the pinned constant. \
         Current hash: {h}"
    );
}
