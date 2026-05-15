//! Claude Code CLI backend for the `LlmProvider` trait.

use super::{extract_json, run_subprocess, LlmError, LlmProvider};

/// Claude Code CLI backend. Invokes the `claude` binary as a subprocess.
pub struct ClaudeProvider {
    /// Model override (e.g. `"claude-opus-4-5"`). `None` uses the CLI default.
    model: Option<String>,
}

impl ClaudeProvider {
    /// Creates a new provider. `model` is forwarded to `--model` if `Some`.
    pub fn new(model: Option<String>) -> Self {
        Self { model }
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
        let raw = run_subprocess("claude", &args, prompt).await?;
        extract_json(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name() {
        // setup / execute
        let provider = ClaudeProvider::new(None);

        // verify
        assert_eq!(provider.name(), "claude");
    }
}
