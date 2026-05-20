//! Account detection for each LLM provider backend.
//!
//! Reads provider-specific config files or runs CLI commands to determine
//! which account is active. Used at startup to log and verify the identity.
//!
//! # Example
//!
//! ```no_run
//! use easy_diff::config::{Provider, ProviderSettings};
//! use easy_diff::llm::account::detect_account;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let settings = ProviderSettings::default();
//! let account = detect_account(&Provider::Claude, &settings).await?;
//! println!("logged in as: {account}");
//! # Ok(())
//! # }
//! ```

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::config::{Provider, ProviderSettings};

/// Detects the active account for the given provider.
///
/// Returns a human-readable account identifier (email address, login
/// status string, etc.) or an error if detection fails.
pub async fn detect_account(provider: &Provider, settings: &ProviderSettings) -> Result<String> {
    match provider {
        Provider::Claude => detect_claude_account(settings),
        Provider::Codex => detect_codex_account(settings).await,
        Provider::Gemini => detect_gemini_account(settings),
    }
}

/// Claude: read `<config_dir>/.claude.json` → `oauthAccount.emailAddress`.
fn detect_claude_account(settings: &ProviderSettings) -> Result<String> {
    let dir = resolve_config_dir(settings.profile.as_deref(), "CLAUDE_CONFIG_DIR", ".claude");
    let path = dir.join(".claude.json");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    json["oauthAccount"]["emailAddress"]
        .as_str()
        .map(String::from)
        .with_context(|| format!("missing oauthAccount.emailAddress in {}", path.display()))
}

/// Codex: run `codex login status` and return the output.
async fn detect_codex_account(settings: &ProviderSettings) -> Result<String> {
    let command = settings.command.as_deref().unwrap_or("codex");
    let envs: Vec<(&str, &str)> = settings
        .profile
        .as_deref()
        .map(|p| vec![("CODEX_HOME", p)])
        .unwrap_or_default();

    let output = super::run_subprocess(command, &["login", "status"], "", &envs)
        .await
        .context("failed to run codex login status")?;

    let trimmed = output.trim().to_string();
    if trimmed.is_empty() {
        bail!("codex login status returned empty output");
    }
    Ok(trimmed)
}

/// Gemini: read `<config_dir>/google_accounts.json` → `active`.
fn detect_gemini_account(settings: &ProviderSettings) -> Result<String> {
    let dir = resolve_config_dir(settings.profile.as_deref(), "GEMINI_CLI_HOME", ".gemini");
    let path = dir.join("google_accounts.json");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    json["active"]
        .as_str()
        .map(String::from)
        .with_context(|| format!("missing 'active' field in {}", path.display()))
}

/// Resolves the config directory: explicit profile path > env var > `~/<default_dot_dir>`.
fn resolve_config_dir(profile: Option<&str>, env_var: &str, default_dot_dir: &str) -> PathBuf {
    if let Some(p) = profile {
        return expand_tilde(p);
    }
    if let Ok(val) = std::env::var(env_var) {
        return PathBuf::from(val);
    }
    dirs::home_dir()
        .map(|h| h.join(default_dot_dir))
        .unwrap_or_else(|| PathBuf::from(default_dot_dir))
}

/// Expands a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn expand_tilde_with_suffix() {
        // setup / execute
        let result = expand_tilde("~/foo/bar");

        // verify
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("foo/bar"));
    }

    #[test]
    fn expand_tilde_bare() {
        // setup / execute
        let result = expand_tilde("~");

        // verify
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home);
    }

    #[test]
    fn expand_tilde_absolute_path_unchanged() {
        // setup / execute
        let result = expand_tilde("/etc/config");

        // verify
        assert_eq!(result, Path::new("/etc/config"));
    }

    #[test]
    fn resolve_config_dir_profile_takes_priority() {
        // setup / execute
        let result = resolve_config_dir(Some("/custom/dir"), "NONEXISTENT_VAR_12345", ".default");

        // verify
        assert_eq!(result, Path::new("/custom/dir"));
    }

    #[test]
    fn resolve_config_dir_falls_back_to_home() {
        // setup / execute
        let result = resolve_config_dir(None, "NONEXISTENT_VAR_12345", ".myapp");

        // verify
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join(".myapp"));
    }

    #[test]
    fn detect_claude_account_parses_json() {
        // setup
        let dir = tempfile::tempdir().unwrap();
        let json = serde_json::json!({
            "oauthAccount": {
                "emailAddress": "test@example.com",
                "organizationName": "Test Org"
            }
        });
        std::fs::write(dir.path().join(".claude.json"), json.to_string()).unwrap();

        let settings = ProviderSettings {
            profile: Some(dir.path().to_string_lossy().into_owned()),
            ..Default::default()
        };

        // execute
        let result = detect_claude_account(&settings);

        // verify
        assert_eq!(result.unwrap(), "test@example.com");
    }

    #[test]
    fn detect_claude_account_missing_field_errors() {
        // setup
        let dir = tempfile::tempdir().unwrap();
        let json = serde_json::json!({"oauthAccount": {}});
        std::fs::write(dir.path().join(".claude.json"), json.to_string()).unwrap();

        let settings = ProviderSettings {
            profile: Some(dir.path().to_string_lossy().into_owned()),
            ..Default::default()
        };

        // execute
        let result = detect_claude_account(&settings);

        // verify
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("emailAddress"));
    }

    #[test]
    fn detect_gemini_account_parses_json() {
        // setup
        let dir = tempfile::tempdir().unwrap();
        let json = serde_json::json!({
            "active": "user@example.com",
            "old": []
        });
        std::fs::write(dir.path().join("google_accounts.json"), json.to_string()).unwrap();

        let settings = ProviderSettings {
            profile: Some(dir.path().to_string_lossy().into_owned()),
            ..Default::default()
        };

        // execute
        let result = detect_gemini_account(&settings);

        // verify
        assert_eq!(result.unwrap(), "user@example.com");
    }

    #[test]
    fn detect_gemini_account_missing_field_errors() {
        // setup
        let dir = tempfile::tempdir().unwrap();
        let json = serde_json::json!({});
        std::fs::write(dir.path().join("google_accounts.json"), json.to_string()).unwrap();

        let settings = ProviderSettings {
            profile: Some(dir.path().to_string_lossy().into_owned()),
            ..Default::default()
        };

        // execute
        let result = detect_gemini_account(&settings);

        // verify
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("active"));
    }
}
