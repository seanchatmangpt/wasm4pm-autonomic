//! LLM provider bridge — `ainst` admits **untrusted** model output.
//!
//! Boundary doctrine: the model proposes a world shape; AutoInstinct
//! validates and admits it. The Gemini CLI's `--output-format json` only
//! normalizes transport (`{response, stats, error}` envelope). The
//! payload inside `.response` must pass:
//!
//! 1. serde shape gate ([`schema::OcelWorld`])
//! 2. structural integrity ([`admit::validate_structural`])
//! 3. ontology profile purity ([`admit::validate_ontology`])
//! 4. response-lattice closure (canonical 7 from `ccog`)
//!
//! before the world is forwarded to motif discovery.

pub mod admit;
pub mod config;
pub mod gemini_cli;
pub mod schema;

pub use admit::{admit, LlmAdmissionError};
pub use config::LlmConfig;
pub use schema::{Counterfactual, ExpectedInstinct, OcelEvent, OcelObject, OcelWorld};
