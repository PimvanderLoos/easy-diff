//! Claude Code CLI backend for the `LlmProvider` trait.
#![allow(dead_code)]

use super::{account::expand_tilde, extract_json, run_subprocess, LlmError, LlmProvider};

/// Claude Code CLI backend. Invokes the `claude` binary as a subprocess.
pub struct ClaudeProvider {
    /// Command name or path for the Claude CLI binary.
    command: String,
    /// Alternate config directory (tilde-expanded). Passed as `CLAUDE_CONFIG_DIR` env var.
    profile: Option<String>,
    /// Model override (e.g. `"claude-opus-4-5"`). `None` uses the CLI default.
    model: Option<String>,
}

impl ClaudeProvider {
    /// Creates a new provider.
    pub fn new(command: Option<String>, profile: Option<String>, model: Option<String>) -> Self {
        Self {
            command: command.unwrap_or_else(|| "claude".to_string()),
            profile: profile.map(|p| expand_tilde(&p).to_string_lossy().into_owned()),
            model,
        }
    }
}

impl LlmProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    async fn analyze(
        &self,
        prompt: &str,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let mut args = vec!["--print", "--output-format", "json"];
        let model_flag;
        if let Some(model) = &self.model {
            model_flag = model.clone();
            args.extend_from_slice(&["--model", &model_flag]);
        }
        let envs: Vec<(&str, &str)> = self
            .profile
            .as_deref()
            .map(|p| vec![("CLAUDE_CONFIG_DIR", p)])
            .unwrap_or_default();
        let raw = run_subprocess(&self.command, &args, prompt, &envs).await?;
        let envelope = extract_json(&raw)?;
        unwrap_claude_envelope(envelope)
    }
}

/// Unwraps the Claude Code CLI JSON envelope to extract the model's response.
///
/// When invoked with `--output-format json`, the CLI wraps the model output in
/// an envelope with metadata fields (`is_error`, `modelUsage`, `session_id`, etc.).
/// The actual model response lives in the `"result"` field as an escaped JSON string.
///
/// If the envelope reports `is_error: true`, the `"result"` field holds a
/// human-readable error message rather than JSON; this returns a clear
/// [`LlmError::InvalidJson`] instead of letting the message fail to parse downstream.
/// If the parsed value doesn't look like an envelope (no `"result"` string field),
/// it's returned as-is (the model may have output raw JSON directly).
fn unwrap_claude_envelope(value: serde_json::Value) -> Result<serde_json::Value, LlmError> {
    if value.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
        let message = value
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(LlmError::InvalidJson {
            message: format!("claude CLI reported an error: {message}"),
        });
    }

    if let Some(result_str) = value.get("result").and_then(|v| v.as_str()) {
        extract_json(result_str)
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name() {
        // setup / execute
        let provider = ClaudeProvider::new(None, None, None);

        // verify
        assert_eq!(provider.name(), "claude");
    }

    #[test]
    fn default_command() {
        // setup / execute
        let provider = ClaudeProvider::new(None, None, None);

        // verify
        assert_eq!(provider.command, "claude");
    }

    #[test]
    fn custom_command() {
        // setup / execute
        let provider = ClaudeProvider::new(Some("/usr/local/bin/claude".into()), None, None);

        // verify
        assert_eq!(provider.command, "/usr/local/bin/claude");
    }

    #[test]
    fn unwrap_envelope_extracts_result_field() {
        // setup
        let envelope = serde_json::json!({
            "is_error": false,
            "result": "{\"summary\":\"test summary\",\"change_types\":[\"feature\"]}",
            "session_id": "abc-123",
            "type": "result"
        });

        // execute
        let extracted = unwrap_claude_envelope(envelope).unwrap();

        // verify
        assert_eq!(extracted["summary"], "test summary");
        assert_eq!(extracted["change_types"][0], "feature");
    }

    #[test]
    fn unwrap_envelope_passes_through_raw_json() {
        // setup
        let raw = serde_json::json!({
            "summary": "direct output",
            "change_types": ["bug-fix"]
        });

        // execute
        let extracted = unwrap_claude_envelope(raw).unwrap();

        // verify
        assert_eq!(extracted["summary"], "direct output");
    }

    #[test]
    fn unwrap_envelope_extracts_fenced_result() {
        // setup
        let envelope = serde_json::json!({
            "is_error": false,
            "result": "```json\n{\"k\":1}\n```"
        });

        // execute
        let extracted = unwrap_claude_envelope(envelope).unwrap();

        // verify
        assert_eq!(extracted, serde_json::json!({"k": 1}));
    }

    #[test]
    fn unwrap_envelope_fails_when_is_error() {
        // setup
        let envelope = serde_json::json!({
            "is_error": true,
            "result": "rate limit exceeded"
        });

        // execute
        let result = unwrap_claude_envelope(envelope);

        // verify
        assert!(matches!(result, Err(LlmError::InvalidJson { .. })));
    }
}
