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
//! Module ladder maps to the 10-phase Vision-2030 plan
//! ([`docs/phases.md`](../docs/phases.md)):
//!
//! | Phase | Tier | Modules |
//! |---:|---|---|
//! | 1 | Foundation Compiler | [`corpus`], [`motifs`], [`synth`], [`jtbd`], [`gauntlet`], [`compile`], [`drift`], [`registry`], [`ocel`] |
//! | 2 | Process Mining Substrate | [`petri`], [`align`] |
//! | 3 | Causal & Counterfactual Engine | [`counterfactual`], [`causal`] |
//! | 4 | Hardened Gauntlet | [`metamorphic`], [`receipt_audit`], [`powl64_audit`], [`mutation`] |
//! | 5 | Pack Lifecycle & Registry | [`manifest`], [`lifecycle`] |
//! | 6 | Distributed Runtime Bridge | [`bridge`] |
//! | 7 | Domain Pack Library | [`domain`] |
//! | 8 | LLM World Generation Loop | [`world_gen`] |
//! | 9 | Compliance & Governance | [`compliance`] |
//! | 10 | Continuous Learning at Scale | [`scale`], [`streaming`] |

// Phase 1 — Foundation Compiler.
pub mod corpus;
pub mod motifs;
pub mod synth;
pub mod ocel;
pub mod jtbd;
pub mod gauntlet;
pub mod compile;
pub mod drift;
pub mod registry;

// Phase 2 — Process Mining Substrate.
pub mod petri;
pub mod align;

// Phase 3 — Causal & Counterfactual Engine.
pub mod counterfactual;
pub mod causal;

// Phase 4 — Hardened Gauntlet.
pub mod metamorphic;
pub mod receipt_audit;
pub mod powl64_audit;
pub mod mutation;

// Phase 5 — Pack Lifecycle & Registry.
pub mod manifest;
pub mod lifecycle;

// Phase 6 — Distributed Runtime Bridge.
pub mod bridge;

// Phase 7 — Domain Pack Library.
pub mod domain;

// Phase 8 — LLM World Generation Loop.
pub mod world_gen;

// Phase 9 — Compliance & Governance.
pub mod compliance;

// Phase 10 — Continuous Learning at Scale.
pub mod scale;
pub mod streaming;

// Constitutional doctrine — programmatic invariants from `SPR.md`.
pub mod doctrine;

// Doctrine-to-code coverage table (Kill Zone 1 of the anti-fake gauntlet).
pub mod doctrine_coverage;

// LLM provider bridge (Gemini CLI; pluggable). Untrusted output goes
// through strict admission before becoming corpus.
pub mod llm;

/// AutoInstinct semver string used in compiled field-pack metadata.
pub const AUTOINSTINCT_VERSION: &str = "30.1.1";

/// Re-exports the canonical `ccog` response classes — AutoInstinct never
/// forks the lattice.
pub use ccog::instinct::AutonomicInstinct;
