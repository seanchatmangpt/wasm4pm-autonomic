//! Synchronous wrapper around the Gemini CLI.
//!
//! Invokes `gemini -m <model> -p <prompt> --output-format json` and
//! returns the **string inside `.response`** of the outer envelope. The
//! returned string is *untrusted*; callers must pass it through
//! [`crate::llm::admit::admit`] before using.

use std::io::Write;
use std::process::{Command, Stdio};

use thiserror::Error;

use crate::llm::config::LlmConfig;

/// Failure modes when shelling out to `gemini`.
#[derive(Debug, Error)]
pub enum GeminiError {
    /// Could not spawn the `gemini` binary at all.
    #[error("spawn `gemini` failed: {0}")]
    Spawn(String),
    /// Gemini exited non-zero.
    #[error("gemini exited {code:?}: {stderr}")]
    Exit {
        /// Non-zero exit code, if any.
        code: Option<i32>,
        /// Captured stderr.
        stderr: String,
    },
    /// Outer JSON envelope failed to parse.
    #[error("envelope parse: {0}")]
    Envelope(String),
    /// Envelope contained a non-null `error` field.
    #[error("gemini reported error: {0}")]
    Reported(String),
    /// Envelope had no `response` string.
    #[error("envelope missing string `.response`")]
    MissingResponse,
}

/// Call `gemini -m <model> -p <prompt> --output-format json` and return
/// `.response` as a raw string (still untrusted).
pub fn call_response(
    cfg: &LlmConfig,
    prompt: &str,
    stdin_body: Option<&str>,
) -> Result<String, GeminiError> {
    let mut child = Command::new("gemini")
        .arg("-m")
        .arg(&cfg.model)
        .arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GeminiError::Spawn(e.to_string()))?;

    if let Some(body) = stdin_body {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(body.as_bytes())
                .map_err(|e| GeminiError::Spawn(e.to_string()))?;
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| GeminiError::Spawn(e.to_string()))?;

    if !output.status.success() {
        return Err(GeminiError::Exit {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    parse_envelope(&output.stdout)
}

fn parse_envelope(stdout: &[u8]) -> Result<String, GeminiError> {
    let envelope: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| GeminiError::Envelope(e.to_string()))?;
    if let Some(err) = envelope.get("error") {
        if !err.is_null() {
            return Err(GeminiError::Reported(err.to_string()));
        }
    }
    envelope
        .get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(GeminiError::MissingResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_extracts_response_string() {
        let raw = br#"{"response":"{\"version\":\"30.1.1\"}","stats":{},"error":null}"#;
        let r = parse_envelope(raw).unwrap();
        assert_eq!(r, r#"{"version":"30.1.1"}"#);
    }

    #[test]
    fn envelope_with_non_null_error_is_reported() {
        let raw = br#"{"response":null,"stats":{},"error":"rate limited"}"#;
        let err = parse_envelope(raw).unwrap_err();
        assert!(matches!(err, GeminiError::Reported(_)));
    }

    #[test]
    fn envelope_missing_response_is_rejected() {
        let raw = br#"{"stats":{},"error":null}"#;
        assert!(matches!(parse_envelope(raw), Err(GeminiError::MissingResponse)));
    }
}
