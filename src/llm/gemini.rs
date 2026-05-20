//! Gemini CLI backend for the `LlmProvider` trait.
// Not yet called from main.rs — wired in Epic 3.
#![allow(dead_code)]
//!
//! Unlike Claude and Codex, the Gemini CLI does not guarantee structured JSON
//! output. [`GeminiProvider`] validates the response against the caller-supplied
//! schema and retries up to [`GeminiProvider::max_retries`] times, appending
//! correction context to the prompt on each failed attempt.

use super::{extract_json, run_subprocess, validate_against_schema, LlmError, LlmProvider};

/// Only the first N validation errors are appended to the retry prompt to avoid
/// blowing up the model's context window when the response is very wrong.
const MAX_ERRORS_IN_RETRY: usize = 5;

/// Gemini CLI backend. Invokes the `gemini` binary as a subprocess and retries
/// up to `max_retries` times on schema validation failure.
pub struct GeminiProvider {
    /// Model override (e.g. `"gemini-2.0-flash"`). `None` uses the CLI default.
    model: Option<String>,
    /// Maximum total attempts (including the first). Clamped to at least 1.
    max_retries: u32,
}

impl GeminiProvider {
    /// Creates a new provider.
    ///
    /// `max_retries` must be at least 1. If 0 is passed it is silently clamped
    /// to 1 so the first attempt is always made.
    pub fn new(model: Option<String>, max_retries: u32) -> Self {
        Self {
            model,
            max_retries: max_retries.max(1),
        }
    }
}

impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let model = self.model.clone();
        retry_loop(prompt, schema, self.max_retries, move |current_prompt| {
            let model = model.clone();
            async move {
                let mut args: Vec<&str> = vec![];
                let model_flag;
                if let Some(ref m) = model {
                    model_flag = m.clone();
                    args.extend_from_slice(&["--model", &model_flag]);
                }
                run_subprocess("gemini", &args, &current_prompt, &[]).await
            }
        })
        .await
    }
}

/// Core retry loop, parameterized over the fetch function.
///
/// Allows unit testing without spawning subprocesses. `fetch` receives the
/// current prompt (with correction context appended on retries) and returns
/// the raw stdout string.
async fn retry_loop<F, Fut>(
    prompt: &str,
    schema: &serde_json::Value,
    max_retries: u32,
    fetch: F,
) -> Result<serde_json::Value, LlmError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<String, LlmError>>,
{
    let mut current_prompt = prompt.to_string();
    let mut last_errors: Vec<String> = Vec::new();

    for attempt in 1..=max_retries {
        let raw = match fetch(current_prompt.clone()).await {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let value = match extract_json(&raw) {
            Ok(v) => v,
            Err(e) => {
                last_errors = vec![e.to_string()];
                if attempt < max_retries {
                    current_prompt = build_retry_prompt(prompt, &last_errors);
                }
                continue;
            }
        };

        let errors = validate_against_schema(&value, schema);
        if errors.is_empty() {
            return Ok(value);
        }

        last_errors = errors;
        if attempt < max_retries {
            current_prompt = build_retry_prompt(prompt, &last_errors);
        }
    }

    Err(LlmError::ValidationFailed {
        attempts: max_retries,
        errors: last_errors,
    })
}

/// Appends a correction instruction to the original prompt, listing the
/// schema validation errors from the previous attempt (capped at
/// [`MAX_ERRORS_IN_RETRY`]).
fn build_retry_prompt(original: &str, errors: &[String]) -> String {
    let error_list = errors
        .iter()
        .take(MAX_ERRORS_IN_RETRY)
        .enumerate()
        .map(|(i, e)| format!("  {}. {e}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{original}\n\n\
        Your previous response did not conform to the required JSON schema. \
        Please correct the following errors and respond with valid JSON only:\n\
        {error_list}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn name_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        })
    }

    #[tokio::test]
    async fn retry_loop_succeeds_on_first_attempt() {
        // setup
        let schema = name_schema();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // execute
        let result = retry_loop("prompt", &schema, 3, move |_| {
            calls_c.fetch_add(1, Ordering::SeqCst);
            async { Ok(r#"{"name":"alice"}"#.to_string()) }
        })
        .await;

        // verify
        assert_eq!(result.unwrap(), json!({"name": "alice"}));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_loop_retries_on_invalid_json() {
        // setup
        let schema = name_schema();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // execute
        let result = retry_loop("prompt", &schema, 3, move |_| {
            let n = calls_c.fetch_add(1, Ordering::SeqCst);
            async move {
                if n < 2 {
                    Ok("not json".to_string())
                } else {
                    Ok(r#"{"name":"bob"}"#.to_string())
                }
            }
        })
        .await;

        // verify
        assert_eq!(result.unwrap(), json!({"name": "bob"}));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_loop_retries_on_schema_violation() {
        // setup
        let schema = name_schema();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // execute
        let result = retry_loop("prompt", &schema, 3, move |_| {
            let n = calls_c.fetch_add(1, Ordering::SeqCst);
            async move {
                if n < 2 {
                    Ok(r#"{"wrong_field": 1}"#.to_string())
                } else {
                    Ok(r#"{"name":"carol"}"#.to_string())
                }
            }
        })
        .await;

        // verify
        assert_eq!(result.unwrap(), json!({"name": "carol"}));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_loop_exhausts_retries() {
        // setup
        let schema = name_schema();

        // execute
        let result = retry_loop("prompt", &schema, 3, |_| async {
            Ok(r#"{"wrong_field": 1}"#.to_string())
        })
        .await;

        // verify
        let err = result.unwrap_err();
        assert!(
            matches!(err, LlmError::ValidationFailed { attempts: 3, .. }),
            "expected ValidationFailed with attempts=3, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn retry_loop_stops_on_process_error() {
        // setup
        let schema = name_schema();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // execute
        let result = retry_loop("prompt", &schema, 3, move |_| {
            calls_c.fetch_add(1, Ordering::SeqCst);
            async {
                Err(LlmError::ProcessFailed {
                    code: 1,
                    stderr: "binary not found".to_string(),
                })
            }
        })
        .await;

        // verify
        assert!(
            matches!(result, Err(LlmError::ProcessFailed { code: 1, .. })),
            "expected ProcessFailed to propagate unchanged"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "should not retry on process error"
        );
    }

    #[test]
    fn build_retry_prompt_includes_errors() {
        // setup
        let original = "Analyze this diff";
        let errors = vec![
            "missing field 'name'".to_string(),
            "wrong type at /count".to_string(),
        ];

        // execute
        let result = build_retry_prompt(original, &errors);

        // verify
        assert!(result.starts_with(original));
        assert!(result.contains("1. missing field 'name'"));
        assert!(result.contains("2. wrong type at /count"));
        assert!(result.contains("JSON schema"));
    }
}
