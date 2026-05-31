//! Codex CLI backend for the `LlmProvider` trait.
#![allow(dead_code)]

use super::{account::expand_tilde, extract_json, run_subprocess, LlmError, LlmProvider};

/// Codex CLI backend. Invokes the `codex` binary as a subprocess.
pub struct CodexProvider {
    /// Command name or path for the Codex CLI binary.
    command: String,
    /// Alternate config directory (tilde-expanded). Passed as `CODEX_HOME` env var.
    profile: Option<String>,
    /// Model override (e.g. `"o4-mini"`). `None` uses the CLI default.
    model: Option<String>,
}

impl CodexProvider {
    /// Creates a new provider.
    pub fn new(command: Option<String>, profile: Option<String>, model: Option<String>) -> Self {
        Self {
            command: command.unwrap_or_else(|| "codex".to_string()),
            profile: profile.map(|p| expand_tilde(&p).to_string_lossy().into_owned()),
            model,
        }
    }
}

impl LlmProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    async fn analyze(
        &self,
        prompt: &str,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let mut args = vec!["--quiet"];
        let model_flag;
        if let Some(model) = &self.model {
            model_flag = model.clone();
            args.extend_from_slice(&["--model", &model_flag]);
        }
        let envs: Vec<(&str, &str)> = self
            .profile
            .as_deref()
            .map(|p| vec![("CODEX_HOME", p)])
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
        let provider = CodexProvider::new(None, None, None);

        // verify
        assert_eq!(provider.name(), "codex");
    }

    #[test]
    fn default_command() {
        // setup / execute
        let provider = CodexProvider::new(None, None, None);

        // verify
        assert_eq!(provider.command, "codex");
    }
}
