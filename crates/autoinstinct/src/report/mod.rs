//! Phase 6 / KZ8 — Evidence-to-Report Compiler.
//!
//! Transforms an anti-fake evidence bundle into an admitted, narrated
//! report bundle. The admission layer ([`admit::admit`]) is load-bearing;
//! the prompt is hint-only. No claim survives without a cited evidence
//! file and a verified snippet.

pub mod admit;
pub mod diff;
pub mod evidence;
pub mod gemini;
pub mod prompt;
pub mod render;
pub mod schema;

pub use admit::{admit, ReportAdmissionError, FORBIDDEN_OVERCLAIMS};
pub use diff::{diff, DimensionChange, ScorecardDiff};
pub use evidence::{EvidenceBundle, EvidenceLoadError, REQUIRED_FILES};
pub use gemini::{generate, parse_response, ReportError};
pub use prompt::build_prompt;
pub use render::{write_report, RenderError};
pub use schema::{
    GeneratedReport, OpenRisk, ReportClaim, ReportKind, ReportStatus, RiskSeverity,
};
