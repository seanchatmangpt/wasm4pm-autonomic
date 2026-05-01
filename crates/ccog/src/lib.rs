#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![recursion_limit = "2048"]
#![cfg_attr(
    feature = "nightly",
    feature(portable_simd, generic_const_exprs, const_trait_impl)
)]
#![cfg_attr(feature = "nightly", allow(incomplete_features))]

//! Compiled Cognition core: field-cognition facade over RDF graph closure.

pub mod admit;
pub mod bark_artifact;
pub mod bark_kernel;
pub mod compiled;
pub mod compiled_hook;
pub mod conformance;
pub mod construct8;
pub mod export;
pub mod facade;
pub mod field;
pub mod graph;
pub mod hooks;
pub mod ids;
pub mod instinct;
pub mod macros;
pub mod mask;
pub mod multimodal;
pub mod operation;
pub mod packs;
pub mod powl;
pub mod powl64;
pub mod receipt;
pub mod trace;
pub mod utils;
pub mod verdict;

pub mod breeds {
    //! Cognitive breed passes: ELIZA, MYCIN, STRIPS, SHRDLU, Prolog, Hearsay-II,
    //! DENDRAL, plus Phase-9 expansions GPS / SOAR / PRS / CBR.
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
}

pub mod abi;

pub mod runtime;

pub use bark_artifact::{bark, bark_table, BarkSlot, BUILTINS};
pub use bark_kernel::BarkKernel;
pub use compiled::CompiledFieldSnapshot;
pub use compiled_hook::{compile_builtin, compute_present_mask, CompiledHook, CompiledHookTable};
pub use construct8::Construct8;
pub use facade::process;
pub use facade::process_with_hooks;
pub use field::FieldContext;
pub use hooks::{HookOutcome, HookRegistry, HookTrigger, KnowledgeHook};
pub use receipt::Receipt;
pub use runtime::delta::{GraphDelta, GraphSnapshot};
pub use runtime::event::{CaseId, Event, Lifecycle};
pub use runtime::posture::PostureMachine;
pub use runtime::scheduler::{Scheduler, TickReport};
pub use runtime::step::{Runtime, StepReport};
pub use verdict::{
    AffordanceVerdict, Breed, PackPosture, PlanAdmission, PlanVerdict, ProvenanceChain,
    ProvenanceStep, RelationProof, Verdict,
};

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
