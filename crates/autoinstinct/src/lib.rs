#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! AutoInstinct v30.1.1 — trace-to-instinct compilation.
//!
//! AutoInstinct is the compiler layer above [`ccog`]. It learns lawful
//! response policies from proof-backed traces, OCEL worlds, public
//! ontology profiles, and adversarial JTBD tests, then compiles admitted
//! policies into deployable field packs.
//!
//! Governing law: `A = μ(O*)`. Raw observation does not authorize action.
//! Action is projected from closed context.
//!
//! Pipeline:
//!
//! ```text
//! ontology profile
//! → OCEL worlds
//! → trace corpus
//! → motif discovery
//! → candidate μ policy
//! → generated JTBD tests
//! → gauntlet
//! → compiled field pack
//! → ccog deployment
//! ```
//!
//! Modules map to README §"Suggested future modules":
//!
//! - [`corpus`] — trace ingestion and indexing.
//! - [`motifs`] — recurring response motifs over closed contexts.
//! - [`synth`] — candidate μ policy synthesis.
//! - [`ocel`] — Object-Centric Event Log generation + validation.
//! - [`jtbd`] — generated JTBD scenario emission.
//! - [`gauntlet`] — admit/deny gate (positive + negative + perturbation).
//! - [`compile`] — field-pack compilation.
//! - [`drift`] — outcome monitor / drift detection feedback.
//! - [`registry`] — pack registry + version pinning.

pub mod corpus;
pub mod motifs;
pub mod synth;
pub mod ocel;
pub mod jtbd;
pub mod gauntlet;
pub mod compile;
pub mod drift;
pub mod registry;

/// AutoInstinct semver string used in compiled field-pack metadata.
pub const AUTOINSTINCT_VERSION: &str = "30.1.1";

/// Re-exports the canonical `ccog` response classes — AutoInstinct never
/// forks the lattice.
pub use ccog::instinct::AutonomicInstinct;
