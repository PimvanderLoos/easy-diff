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
        extract_json(&raw)
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
}
