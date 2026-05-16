//! LLM provider abstraction: `LlmProvider` trait, provider dispatch, and
//! the `LlmDispatcher` that selects between a default and optional fallback provider.
//!
//! The [`LlmProvider`] trait is implemented by each CLI backend. [`AnyProvider`]
//! is an enum that delegates to the concrete provider chosen at startup, avoiding
//! `Box<dyn LlmProvider>` and the `async-trait` crate. [`LlmDispatcher`] wraps
//! default + optional fallback and retries on any [`LlmError`].
//!
//! [`validate_against_schema`] is a shared helper used by backends that need to
//! verify LLM output conforms to a JSON Schema (notably the Gemini backend).
//!
//! # Example
//! ```no_run
//! use easy_diff::llm::{LlmProvider, create_dispatcher};
//! use easy_diff::config::Config;
//!
//! // let config: Config = ...;
//! // let dispatcher = create_dispatcher(&config);
//! // let value = dispatcher.analyze("prompt", &schema).await?;
//! ```

pub mod claude;
pub mod codex;
pub mod debug;
pub mod gemini;
pub mod schema;

/// Errors produced by LLM provider backends.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM process failed to start: {message}")]
    ProcessStart { message: String },
    #[error("LLM process exited with status {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },
    #[error("LLM returned empty output")]
    EmptyOutput,
    #[error("LLM output is not valid JSON: {message}")]
    InvalidJson { message: String },
    #[error("LLM output failed schema validation after {attempts} attempt(s): {errors:?}")]
    ValidationFailed { attempts: u32, errors: Vec<String> },
    #[error("I/O error communicating with LLM process: {0}")]
    Io(#[from] std::io::Error),
}

/// Maximum bytes of LLM output included in log messages (10KB).
const MAX_LOG_LEN: usize = 10_240;

/// Implemented by each CLI backend.
///
/// `analyze` takes a plaintext prompt and the JSON Schema for the expected
/// response shape; it returns the validated JSON value. Implementations are
/// responsible for retrying on validation failure as appropriate for their backend.
pub trait LlmProvider: Send + Sync {
    /// Short identifier shown in error messages and logs (e.g. `"claude"`).
    fn name(&self) -> &str;

    /// Sends `prompt` to the LLM and returns a [`serde_json::Value`] that
    /// conforms to `schema`.
    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError>;
}

/// Concrete provider selected from config at startup.
///
/// Delegates to the appropriate CLI backend without dynamic dispatch.
pub enum AnyProvider {
    /// Claude Code CLI backend.
    Claude(claude::ClaudeProvider),
    /// Codex CLI backend.
    Codex(codex::CodexProvider),
    /// Gemini CLI backend with schema-validation retry logic.
    Gemini(gemini::GeminiProvider),
}

impl LlmProvider for AnyProvider {
    fn name(&self) -> &str {
        match self {
            Self::Claude(p) => p.name(),
            Self::Codex(p) => p.name(),
            Self::Gemini(p) => p.name(),
        }
    }

    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        match self {
            Self::Claude(p) => p.analyze(prompt, schema).await,
            Self::Codex(p) => p.analyze(prompt, schema).await,
            Self::Gemini(p) => p.analyze(prompt, schema).await,
        }
    }
}

/// Selects and dispatches to the configured LLM provider.
///
/// Tries `default`; on any [`LlmError`] logs a warning and tries `fallback` if
/// configured. If fallback also fails, returns the fallback error.
pub struct LlmDispatcher {
    default: AnyProvider,
    #[allow(dead_code)]
    fallback: Option<AnyProvider>,
}

impl LlmDispatcher {
    /// Sends `prompt` to the default provider; falls back to the secondary on error.
    pub async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        match self.default.analyze(prompt, schema).await {
            Ok(v) => Ok(v),
            Err(e) => {
                if let Some(fb) = &self.fallback {
                    tracing::warn!(
                        error = %e,
                        fallback = fb.name(),
                        "primary provider failed, trying fallback"
                    );
                    fb.analyze(prompt, schema).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Returns the name of the active (default) provider.
    pub fn provider_name(&self) -> &str {
        self.default.name()
    }
}

/// Constructs an [`LlmDispatcher`] from resolved config.
///
/// Per-provider settings (model, command) are read from the config sub-tables.
pub fn create_dispatcher(config: &crate::config::Config) -> LlmDispatcher {
    let default = provider_from_config(&config.llm.default_provider, &config.llm);
    let fallback = config
        .llm
        .fallback_provider
        .as_ref()
        .map(|p| provider_from_config(p, &config.llm));
    LlmDispatcher { default, fallback }
}

fn provider_from_config(
    provider: &crate::config::Provider,
    llm: &crate::config::LlmConfig,
) -> AnyProvider {
    match provider {
        crate::config::Provider::Claude => {
            AnyProvider::Claude(claude::ClaudeProvider::new(llm.claude.model.clone()))
        }
        crate::config::Provider::Codex => {
            AnyProvider::Codex(codex::CodexProvider::new(llm.codex.model.clone()))
        }
        crate::config::Provider::Gemini => AnyProvider::Gemini(gemini::GeminiProvider::new(
            llm.gemini.model.clone(),
            llm.max_retries,
        )),
    }
}

/// Spawns `program` with `args`, writes `input` to its stdin, and returns
/// the stdout string. Maps process failures to [`LlmError`].
#[allow(dead_code)]
pub(crate) async fn run_subprocess(
    program: &str,
    args: &[&str],
    input: &str,
) -> Result<String, LlmError> {
    use tokio::io::AsyncWriteExt as _;

    let mut child = tokio::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| LlmError::ProcessStart {
            message: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(LlmError::ProcessFailed { code, stderr });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() {
        return Err(LlmError::EmptyOutput);
    }

    if stdout.len() > MAX_LOG_LEN {
        tracing::debug!(
            program,
            len = stdout.len(),
            truncated = &stdout[..MAX_LOG_LEN],
            "subprocess stdout (truncated)"
        );
    } else {
        tracing::debug!(program, stdout = %stdout, "subprocess stdout");
    }

    Ok(stdout)
}

/// Extracts the first JSON object or array from `text`, stripping markdown code
/// fences if present. Returns [`LlmError::InvalidJson`] if none is found.
#[allow(dead_code)]
pub(crate) fn extract_json(text: &str) -> Result<serde_json::Value, LlmError> {
    let stripped = if let Some(inner) = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
    {
        inner.trim_end_matches("```").trim()
    } else {
        text.trim()
    };

    let start = stripped.find(['{', '[']).unwrap_or(0);

    let value: serde_json::Value =
        serde_json::from_str(&stripped[start..]).map_err(|e| LlmError::InvalidJson {
            message: e.to_string(),
        })?;

    let json_str = value.to_string();
    if json_str.len() > MAX_LOG_LEN {
        tracing::trace!(
            len = json_str.len(),
            "extracted JSON (truncated, >{MAX_LOG_LEN} bytes)"
        );
    } else {
        tracing::trace!(json = %json_str, "extracted JSON");
    }

    Ok(value)
}

/// Validates `value` against `schema`.
///
/// Returns the list of validation error messages, or an empty [`Vec`] if the
/// value is valid. If the schema itself is invalid, returns a single-element
/// Vec describing the schema error.
#[allow(dead_code)]
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> Vec<String> {
    match jsonschema::validator_for(schema) {
        Err(e) => vec![format!("invalid schema: {e}")],
        Ok(validator) => validator
            .iter_errors(value)
            .map(|e| e.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn string_field_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        })
    }

    #[test]
    fn validate_passes_for_valid_value() {
        // setup
        let schema = string_field_schema();
        let value = json!({"name": "alice"});

        // execute
        let errors = validate_against_schema(&value, &schema);

        // verify
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn validate_fails_for_missing_required_field() {
        // setup
        let schema = string_field_schema();
        let value = json!({});

        // execute
        let errors = validate_against_schema(&value, &schema);

        // verify
        assert!(
            !errors.is_empty(),
            "expected validation errors for missing required field"
        );
    }

    #[test]
    fn validate_fails_for_wrong_type() {
        // setup
        let schema = string_field_schema();
        let value = json!({"name": 42});

        // execute
        let errors = validate_against_schema(&value, &schema);

        // verify
        assert!(
            !errors.is_empty(),
            "expected validation errors for wrong type"
        );
    }

    // --- extract_json ---

    #[test]
    fn extract_json_bare_object() {
        // setup
        let input = r#"{"key":"val"}"#;

        // execute
        let result = extract_json(input);

        // verify
        assert_eq!(result.unwrap(), json!({"key": "val"}));
    }

    #[test]
    fn extract_json_fenced_json() {
        // setup
        let input = "```json\n{\"k\":1}\n```";

        // execute
        let result = extract_json(input);

        // verify
        assert_eq!(result.unwrap(), json!({"k": 1}));
    }

    #[test]
    fn extract_json_fenced_no_lang() {
        // setup
        let input = "```\n{\"k\":1}\n```";

        // execute
        let result = extract_json(input);

        // verify
        assert_eq!(result.unwrap(), json!({"k": 1}));
    }

    #[test]
    fn extract_json_with_preamble() {
        // setup
        let input = r#"Here is the output: {"k":1}"#;

        // execute
        let result = extract_json(input);

        // verify
        assert_eq!(result.unwrap(), json!({"k": 1}));
    }

    #[test]
    fn extract_json_invalid() {
        // setup
        let input = "not json at all";

        // execute
        let result = extract_json(input);

        // verify
        assert!(matches!(result, Err(LlmError::InvalidJson { .. })));
    }

    // --- dispatcher / provider factory ---

    #[test]
    fn provider_from_config_maps_all_variants() {
        // setup
        let llm = crate::config::LlmConfig {
            default_provider: crate::config::Provider::Claude,
            fallback_provider: None,
            max_retries: 3,
            timeout_seconds: 300,
            claude: Default::default(),
            codex: Default::default(),
            gemini: Default::default(),
        };

        // execute
        let claude = provider_from_config(&crate::config::Provider::Claude, &llm);
        let codex = provider_from_config(&crate::config::Provider::Codex, &llm);
        let gemini = provider_from_config(&crate::config::Provider::Gemini, &llm);

        // verify
        assert_eq!(claude.name(), "claude");
        assert_eq!(codex.name(), "codex");
        assert_eq!(gemini.name(), "gemini");
    }
}
