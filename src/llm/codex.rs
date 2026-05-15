//! Codex CLI backend for the `LlmProvider` trait.
// Not yet called from main.rs — wired in Epic 3.
#![allow(dead_code)]

use super::{extract_json, run_subprocess, LlmError, LlmProvider};

/// Codex CLI backend. Invokes the `codex` binary as a subprocess.
pub struct CodexProvider {
    /// Model override (e.g. `"o4-mini"`). `None` uses the CLI default.
    model: Option<String>,
}

impl CodexProvider {
    /// Creates a new provider. `model` is forwarded to `--model` if `Some`.
    pub fn new(model: Option<String>) -> Self {
        Self { model }
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
        let raw = run_subprocess("codex", &args, prompt).await?;
        extract_json(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name() {
        // setup / execute
        let provider = CodexProvider::new(None);

        // verify
        assert_eq!(provider.name(), "codex");
    }
}
