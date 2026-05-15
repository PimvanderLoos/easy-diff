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
/// PR-1 adds `Claude` and `Codex` variants; PR-2 completes `Gemini`.
pub enum AnyProvider {}

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
}
