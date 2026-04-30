//! Runtime model selection for the LLM bridge.
//!
//! Priority chain: `--model` flag → `AINST_LLM_MODEL` env → default.

/// Default Gemini model when no override is supplied.
pub const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite-preview";

/// Environment variable consulted when no `--model` flag is provided.
pub const MODEL_ENV: &str = "AINST_LLM_MODEL";

/// Resolved LLM configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmConfig {
    /// Model identifier passed to `gemini -m <model>`.
    pub model: String,
}

impl LlmConfig {
    /// Resolve from optional CLI flag, falling back to env then default.
    #[must_use]
    pub fn resolve(model_flag: Option<String>) -> Self {
        let model = model_flag
            .or_else(|| std::env::var(MODEL_ENV).ok())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Self { model }
    }
}

/// Pure resolution variant for testing: explicit env value lets us
/// avoid `std::env` global state, which is flaky under parallel tests.
#[must_use]
pub fn resolve_with(model_flag: Option<String>, env_value: Option<String>) -> LlmConfig {
    let model = model_flag
        .or(env_value)
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    LlmConfig { model }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_wins_over_env() {
        let cfg = resolve_with(Some("from-flag".into()), Some("from-env".into()));
        assert_eq!(cfg.model, "from-flag");
    }

    #[test]
    fn env_wins_when_no_flag() {
        let cfg = resolve_with(None, Some("from-env".into()));
        assert_eq!(cfg.model, "from-env");
    }

    #[test]
    fn default_when_neither() {
        let cfg = resolve_with(None, None);
        assert_eq!(cfg.model, DEFAULT_MODEL);
    }
}
