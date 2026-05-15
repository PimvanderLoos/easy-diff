//! LLM provider abstraction: `LlmProvider` trait and dispatch to Claude,
//! Codex, and Gemini CLI backends.
//!
//! The [`LlmProvider`] trait is implemented by each CLI backend. [`AnyProvider`]
//! is an enum that delegates to the concrete provider chosen at startup, avoiding
//! `Box<dyn LlmProvider>` and the `async-trait` crate.
//!
//! [`validate_against_schema`] is a shared helper used by backends that need to
//! verify LLM output conforms to a JSON Schema (notably the Gemini backend).
//!
//! # Example
//! ```no_run
//! use easy_diff::llm::{LlmProvider, validate_against_schema};
//! use serde_json::json;
//!
//! let schema = json!({"type": "object", "properties": {"result": {"type": "string"}}, "required": ["result"]});
//! // let provider = ...; // constructed from config in PR-3
//! // let value = provider.analyze("prompt", &schema).await?;
//! ```
#![allow(dead_code)]

pub mod claude;
pub mod codex;
pub mod gemini;
pub mod schema;

/// Errors produced by LLM provider backends.
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
    /// Gemini CLI backend (retry logic added in PR-2).
    Gemini(gemini::GeminiProvider),
}

/// Spawns `program` with `args`, writes `input` to its stdin, and returns
/// the stdout string. Maps process failures to [`LlmError`].
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

    Ok(stdout)
}

/// Extracts the first JSON object or array from `text`, stripping markdown code
/// fences if present. Returns [`LlmError::InvalidJson`] if none is found.
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

    serde_json::from_str(&stripped[start..]).map_err(|e| LlmError::InvalidJson {
        message: e.to_string(),
    })
}

/// Validates `value` against `schema`.
///
/// Returns the list of validation error messages, or an empty [`Vec`] if the
/// value is valid. If the schema itself is invalid, returns a single-element
/// Vec describing the schema error.
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
}
