//! Generate a report by calling the Gemini CLI and admitting the result.
//!
//! Reuses [`crate::llm::gemini_cli::call_response`] for transport so the
//! KZ8 layer does not reintroduce a second Gemini bridge. The returned
//! `.response` string is **untrusted**; we parse it into
//! [`GeneratedReport`] and run [`crate::report::admit::admit`] before
//! returning success.

use thiserror::Error;

use crate::llm::config::LlmConfig;
use crate::llm::gemini_cli::{call_response, GeminiError};
use crate::report::admit::{admit, ReportAdmissionError};
use crate::report::evidence::EvidenceBundle;
use crate::report::prompt::build_prompt;
use crate::report::schema::{GeneratedReport, ReportKind};

/// Errors during report generation.
#[derive(Debug, Error)]
pub enum ReportError {
    /// Gemini CLI failure.
    #[error("gemini: {0}")]
    Gemini(#[from] GeminiError),
    /// `.response` was not valid JSON for [`GeneratedReport`].
    #[error("response parse: {0}")]
    Parse(String),
    /// Admission rejected the report.
    #[error("admission: {0}")]
    Admission(#[from] ReportAdmissionError),
}

/// Generate and admit a report.
///
/// # Errors
///
/// Returns `ReportError::Gemini` for transport failures,
/// `ReportError::Parse` for malformed JSON, and
/// `ReportError::Admission` for KZ8 admission failures.
pub fn generate(
    kind: ReportKind,
    bundle: &EvidenceBundle,
    cfg: &LlmConfig,
) -> Result<GeneratedReport, ReportError> {
    let prompt = build_prompt(kind, bundle);
    let stdin_body = bundle.concat_for_prompt();
    let raw = call_response(cfg, &prompt, Some(&stdin_body))?;
    let report: GeneratedReport = parse_response(&raw)?;
    admit(&report, bundle)?;
    Ok(report)
}

/// Parse the Gemini `.response` string into a [`GeneratedReport`].
///
/// Strips an optional UTF-8 BOM and any markdown fences before parsing.
///
/// # Errors
///
/// Returns `ReportError::Parse` if the trimmed body is not valid JSON
/// for [`GeneratedReport`].
pub fn parse_response(raw: &str) -> Result<GeneratedReport, ReportError> {
    let body = raw.trim_start_matches('\u{feff}').trim();
    let body = body
        .strip_prefix("```json")
        .or_else(|| body.strip_prefix("```"))
        .unwrap_or(body);
    let body = body.strip_suffix("```").unwrap_or(body).trim();
    serde_json::from_str::<GeneratedReport>(body).map_err(|e| ReportError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::schema::{ReportClaim, ReportStatus};

    #[test]
    fn parse_response_strips_markdown_fence() {
        let inner = serde_json::to_string(&GeneratedReport {
            report_kind: ReportKind::Executive,
            title: "t".into(),
            commit: "c".into(),
            toolchain: "tc".into(),
            overall_status: ReportStatus::Pass,
            claims: vec![ReportClaim {
                id: "id".into(),
                claim: "c".into(),
                evidence_files: vec!["scorecard.json".into()],
                evidence_snippets: vec!["overall_pass".into()],
                risk_if_false: "x".into(),
            }],
            open_risks: vec![],
            markdown: "# t".into(),
        })
        .unwrap();
        let raw = format!("```json\n{inner}\n```");
        let r = parse_response(&raw).unwrap();
        assert_eq!(r.title, "t");
    }

    #[test]
    fn parse_response_rejects_garbage() {
        assert!(parse_response("not json").is_err());
    }
}
