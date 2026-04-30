#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(
    feature = "nightly",
    feature(portable_simd, generic_const_exprs, const_trait_impl)
)]
#![cfg_attr(feature = "nightly", allow(incomplete_features))]

//! Compiled Cognition core: field-cognition facade over RDF graph closure.

pub mod admit;
pub mod mask;
pub mod packs;
pub mod bark_artifact;
pub mod bark_kernel;
pub mod compiled;
pub mod compiled_hook;
pub mod graph;
pub mod field;
pub mod instinct;
pub mod multimodal;
pub mod operation;
pub mod verdict;
pub mod receipt;
pub mod facade;
pub mod construct8;
pub mod hooks;
pub mod powl;
pub mod powl64;
pub mod trace;
pub mod conformance;
pub mod utils;
pub mod export;

pub mod breeds {
    //! Cognitive breed passes: ELIZA, MYCIN, STRIPS, SHRDLU, Prolog, Hearsay-II,
    //! DENDRAL, plus Phase-9 expansions GPS / SOAR / PRS / CBR.
    pub mod eliza;
    pub mod mycin;
    pub mod strips;
    pub mod shrdlu;
    pub mod prolog;
    pub mod hearsay;
    pub mod dendral;
    pub mod gps;
    pub mod soar;
    pub mod prs;
    pub mod cbr;
}

pub mod abi;

pub mod runtime;

pub use facade::process;
pub use field::FieldContext;
pub use verdict::{
    AffordanceVerdict, Breed, PackPosture, PlanAdmission, PlanVerdict, ProvenanceChain,
    ProvenanceStep, RelationProof, Verdict,
};
pub use receipt::Receipt;
pub use construct8::Construct8;
pub use hooks::{HookRegistry, KnowledgeHook, HookTrigger, HookOutcome};
pub use compiled::CompiledFieldSnapshot;
pub use compiled_hook::{compile_builtin, compute_present_mask, CompiledHook, CompiledHookTable};
pub use bark_kernel::BarkKernel;
pub use bark_artifact::{bark, bark_table, BarkSlot, BUILTINS};
pub use facade::process_with_hooks;
pub use runtime::scheduler::{Scheduler, TickReport};
pub use runtime::delta::{GraphSnapshot, GraphDelta};
pub use runtime::posture::PostureMachine;
pub use runtime::step::{Runtime, StepReport};

/// Compiled Cognition core: field-cognition facade over RDF graph closure.
///
/// ccog knows what the graph permits the field to do.
///
/// The core formula: `U → O*_U → C_U → A_U → R_U`
///
/// - `U` = bounded operational field
/// - `O*_U` = semantic closure of that field (from RDF graph)
/// - `C_U` = compiled cognition artifact (cognitive pass)
/// - `A_U` = admissible operations
/// - `R_U` = PROV receipt (proof + provenance)
///
/// The MVP proves: phrase binding → missing evidence → blocked transition → admissible operation → receipt.
#[doc(hidden)]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
