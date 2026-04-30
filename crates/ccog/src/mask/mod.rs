//! K-tier mask abstraction (Phase 8).
//!
//! Additive, non-breaking helper tier for mask widths beyond the 64-bit
//! POWL8 ISA. **POWL8 stays on raw `u64`** — the 64-node ISA is
//! constitutionally fixed. Only `compute_present_mask` and a future
//! `decide_kN` family for tables longer than 64 slots may use [`KMask`].
//!
//! # Mask domain rule
//!
//! Mask values produced by this module are **runtime-slot-indexed** unless
//! the constructor explicitly documents otherwise. Plan-node and
//! predicate-bit domains are kept distinct elsewhere in the crate.

pub mod build;
pub mod ktier;
#[cfg(feature = "nightly")]
pub mod simd;

pub use ktier::{KMask, K128, K256, K64};
