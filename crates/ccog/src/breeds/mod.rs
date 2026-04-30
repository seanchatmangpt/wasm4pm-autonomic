//! Cognitive breed templates.
//!
//! Note: this `breeds/mod.rs` is shadowed by the explicit `pub mod breeds { ... }`
//! declaration in `lib.rs` to keep the public crate surface stable. It is kept
//! here for `cargo doc` consistency and future migration to a unified file
//! layout. Phase-9 expansion: `gps`, `soar`, `prs`, `cbr`.

pub mod cbr;
pub mod dendral;
pub mod eliza;
pub mod gps;
pub mod hearsay;
pub mod mycin;
pub mod prolog;
pub mod prs;
pub mod shrdlu;
pub mod soar;
pub mod strips;
