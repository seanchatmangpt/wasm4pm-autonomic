//! Write an admitted [`GeneratedReport`] to disk.
//!
//! No transformations: the JSON is the canonical envelope and the
//! markdown is exactly what Gemini emitted (after admission). The two
//! files are written side-by-side so an auditor can verify the
//! markdown is a faithful render of an admitted JSON envelope.

use std::path::Path;

use crate::report::schema::GeneratedReport;

/// Failures while writing the rendered report.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// I/O failure writing one of the output files.
    #[error("write {path}: {source}")]
    Io {
        /// Offending path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// JSON serialization failure (should not happen for an admitted report).
    #[error("serialize: {0}")]
    Serialize(String),
}

/// Write the admitted report's JSON envelope and markdown body to
/// the supplied paths.
///
/// # Errors
///
/// Returns `RenderError` if either file cannot be written.
pub fn write_report(
    report: &GeneratedReport,
    json_out: &Path,
    md_out: &Path,
) -> Result<(), RenderError> {
    if let Some(parent) = json_out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| RenderError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    if let Some(parent) = md_out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| RenderError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| RenderError::Serialize(e.to_string()))?;
    std::fs::write(json_out, json).map_err(|e| RenderError::Io {
        path: json_out.display().to_string(),
        source: e,
    })?;
    std::fs::write(md_out, &report.markdown).map_err(|e| RenderError::Io {
        path: md_out.display().to_string(),
        source: e,
    })?;
    Ok(())
}
