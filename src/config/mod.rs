//! Configuration loading for global (`~/.config/easy-diff/config.toml`) and
//! per-repository (`.easy-diff/config.toml`) TOML files.
// Public fields are part of the config API consumed by future epics.
#![allow(dead_code)]
//!
//! Config is resolved in three layers (lowest to highest priority):
//! hardcoded defaults → global config → per-repo config.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::Path;
//! use easy_diff::config::load_config;
//!
//! let config = load_config(Some(Path::new("."))).expect("failed to load config");
//! println!("provider: {:?}", config.llm.default_provider);
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Resolved configuration merged from global and per-repository sources.
#[derive(Debug, Clone)]
pub struct Config {
    /// GitHub integration settings.
    pub github: GithubConfig,
    /// Bitbucket integration settings.
    pub bitbucket: BitbucketConfig,
    /// LLM provider settings.
    pub llm: LlmConfig,
    /// Review preferences.
    pub preferences: Preferences,
}

/// GitHub backend selection.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GithubBackend {
    /// Prefer `gh` CLI if available and authenticated, fall back to API/PAT.
    #[default]
    Auto,
    /// Use `gh` CLI exclusively.
    Gh,
    /// Use the REST API with a personal access token exclusively.
    Api,
}

/// GitHub integration settings.
#[derive(Debug, Clone, Default)]
pub struct GithubConfig {
    /// Personal access token for GitHub API calls.
    pub token: Option<String>,
    /// Which backend to use for GitHub operations.
    pub backend: GithubBackend,
}

/// Bitbucket integration settings.
#[derive(Debug, Clone, Default)]
pub struct BitbucketConfig {
    /// Bitbucket account username.
    pub username: Option<String>,
    /// Bitbucket app password (not account password).
    pub app_password: Option<String>,
}

/// Per-provider settings (command path, model override).
#[derive(Debug, Clone, Default)]
pub struct ProviderSettings {
    /// Custom command name or path (e.g. `"gemini"` or `"/usr/local/bin/claude"`).
    pub command: Option<String>,
    /// Model override (e.g. `"gemini-3-pro-preview"`, `"sonnet"`).
    pub model: Option<String>,
}

/// LLM provider settings.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Primary provider used for analysis.
    pub default_provider: Provider,
    /// Fallback provider when the primary is unavailable.
    pub fallback_provider: Option<Provider>,
    /// Maximum retry attempts for schema validation (Gemini). Default: 3.
    pub max_retries: u32,
    /// Subprocess timeout in seconds. Default: 300.
    pub timeout_seconds: u32,
    /// Per-provider settings for Claude.
    pub claude: ProviderSettings,
    /// Per-provider settings for Codex.
    pub codex: ProviderSettings,
    /// Per-provider settings for Gemini.
    pub gemini: ProviderSettings,
}

/// Available LLM provider backends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic Claude Code CLI.
    Claude,
    /// OpenAI Codex CLI.
    Codex,
    /// Google Gemini CLI.
    Gemini,
}

/// Diff review preferences.
#[derive(Debug, Clone)]
pub struct Preferences {
    /// Lines of context around each diff hunk. Default: 5.
    pub context_lines: u32,
    /// Maximum file lines forwarded to the LLM. Default: 500.
    pub max_file_context: u32,
    /// Diff-line count above which a PR is classified as large. Default: 5000.
    pub large_pr_threshold: u32,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// An I/O error while reading a config file.
    #[error("failed to read config file at {path}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The config file exists but contains invalid TOML or unknown fields.
    #[error("invalid config at {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

// ---------------------------------------------------------------------------
// Raw (deserialization) structs — all fields are Option<T>
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawGlobalConfig {
    github: Option<RawGithubConfig>,
    bitbucket: Option<RawBitbucketConfig>,
    llm: Option<RawLlmConfig>,
    preferences: Option<RawPreferencesConfig>,
}

/// Per-repo config — tokens are intentionally absent (global-only).
#[derive(Debug, Deserialize)]
struct RawRepoConfig {
    llm: Option<RawLlmConfig>,
    preferences: Option<RawPreferencesConfig>,
}

#[derive(Debug, Deserialize)]
struct RawGithubConfig {
    token: Option<String>,
    backend: Option<GithubBackend>,
}

#[derive(Debug, Deserialize)]
struct RawBitbucketConfig {
    username: Option<String>,
    app_password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLlmConfig {
    default_provider: Option<Provider>,
    fallback_provider: Option<Provider>,
    max_retries: Option<u32>,
    timeout_seconds: Option<u32>,
    #[serde(alias = "claude-code")]
    claude: Option<RawProviderConfig>,
    codex: Option<RawProviderConfig>,
    gemini: Option<RawProviderConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProviderConfig {
    command: Option<String>,
    profile: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPreferencesConfig {
    context_lines: Option<u32>,
    max_file_context: Option<u32>,
    large_pr_threshold: Option<u32>,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_CONTEXT_LINES: u32 = 5;
const DEFAULT_MAX_FILE_CONTEXT: u32 = 500;
const DEFAULT_LARGE_PR_THRESHOLD: u32 = 5000;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_TIMEOUT_SECONDS: u32 = 300;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads and merges global and per-repository configuration.
///
/// Missing config files produce sensible defaults; invalid TOML returns a
/// [`ConfigError::ParseError`] with the offending file path.
///
/// `repo_root` is the repository working directory used to locate
/// `.easy-diff/config.toml`. Pass `None` to skip per-repo config.
pub fn load_config(repo_root: Option<&Path>) -> Result<Config, ConfigError> {
    let global = load_raw_global()?;
    let repo = match repo_root {
        Some(root) => load_raw_repo(root)?,
        None => None,
    };
    Ok(merge(global, repo))
}

/// Returns the global config file path: `$XDG_CONFIG_HOME/easy-diff/config.toml`.
///
/// Returns `None` if the home directory cannot be determined.
pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("easy-diff").join("config.toml"))
}

/// Returns the per-repository config file path: `<repo_root>/.easy-diff/config.toml`.
pub fn repo_config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".easy-diff").join("config.toml")
}

// ---------------------------------------------------------------------------
// Internal loading
// ---------------------------------------------------------------------------

fn load_raw_global() -> Result<Option<RawGlobalConfig>, ConfigError> {
    match global_config_path() {
        Some(path) => parse_toml(&path),
        None => Ok(None),
    }
}

fn load_raw_repo(repo_root: &Path) -> Result<Option<RawRepoConfig>, ConfigError> {
    parse_toml(&repo_config_path(repo_root))
}

fn parse_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text)
            .map(Some)
            .map_err(|source| ConfigError::ParseError {
                path: path.display().to_string(),
                source,
            }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigError::ReadError {
            path: path.display().to_string(),
            source,
        }),
    }
}

// ---------------------------------------------------------------------------
// Merge: repo overrides global, defaults fill any remaining gaps
// ---------------------------------------------------------------------------

fn merge(global: Option<RawGlobalConfig>, repo: Option<RawRepoConfig>) -> Config {
    let github = GithubConfig {
        token: global
            .as_ref()
            .and_then(|c| c.github.as_ref())
            .and_then(|g| g.token.clone()),
        backend: global
            .as_ref()
            .and_then(|c| c.github.as_ref())
            .and_then(|g| g.backend.clone())
            .unwrap_or_default(),
    };

    let bitbucket = BitbucketConfig {
        username: global
            .as_ref()
            .and_then(|c| c.bitbucket.as_ref())
            .and_then(|b| b.username.clone()),
        app_password: global
            .as_ref()
            .and_then(|c| c.bitbucket.as_ref())
            .and_then(|b| b.app_password.clone()),
    };

    let global_llm = global.as_ref().and_then(|c| c.llm.as_ref());
    let repo_llm = repo.as_ref().and_then(|c| c.llm.as_ref());

    let llm = LlmConfig {
        default_provider: repo_llm
            .and_then(|l| l.default_provider.clone())
            .or_else(|| global_llm.and_then(|l| l.default_provider.clone()))
            .unwrap_or(Provider::Claude),
        fallback_provider: repo_llm
            .and_then(|l| l.fallback_provider.clone())
            .or_else(|| global_llm.and_then(|l| l.fallback_provider.clone())),
        max_retries: repo_llm
            .and_then(|l| l.max_retries)
            .or_else(|| global_llm.and_then(|l| l.max_retries))
            .unwrap_or(DEFAULT_MAX_RETRIES),
        timeout_seconds: repo_llm
            .and_then(|l| l.timeout_seconds)
            .or_else(|| global_llm.and_then(|l| l.timeout_seconds))
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS),
        claude: merge_provider_config(
            global_llm.and_then(|l| l.claude.as_ref()),
            repo_llm.and_then(|l| l.claude.as_ref()),
        ),
        codex: merge_provider_config(
            global_llm.and_then(|l| l.codex.as_ref()),
            repo_llm.and_then(|l| l.codex.as_ref()),
        ),
        gemini: merge_provider_config(
            global_llm.and_then(|l| l.gemini.as_ref()),
            repo_llm.and_then(|l| l.gemini.as_ref()),
        ),
    };

    let global_prefs = global.as_ref().and_then(|c| c.preferences.as_ref());
    let repo_prefs = repo.as_ref().and_then(|c| c.preferences.as_ref());

    let preferences = Preferences {
        context_lines: repo_prefs
            .and_then(|p| p.context_lines)
            .or_else(|| global_prefs.and_then(|p| p.context_lines))
            .unwrap_or(DEFAULT_CONTEXT_LINES),
        max_file_context: repo_prefs
            .and_then(|p| p.max_file_context)
            .or_else(|| global_prefs.and_then(|p| p.max_file_context))
            .unwrap_or(DEFAULT_MAX_FILE_CONTEXT),
        large_pr_threshold: repo_prefs
            .and_then(|p| p.large_pr_threshold)
            .or_else(|| global_prefs.and_then(|p| p.large_pr_threshold))
            .unwrap_or(DEFAULT_LARGE_PR_THRESHOLD),
    };

    Config {
        github,
        bitbucket,
        llm,
        preferences,
    }
}

fn merge_provider_config(
    global: Option<&RawProviderConfig>,
    repo: Option<&RawProviderConfig>,
) -> ProviderSettings {
    ProviderSettings {
        command: repo
            .and_then(|p| p.command.clone())
            .or_else(|| global.and_then(|p| p.command.clone())),
        model: repo
            .and_then(|p| p.model.clone())
            .or_else(|| global.and_then(|p| p.model.clone())),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_temp(content: &str) -> (tempfile::NamedTempFile, PathBuf) {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(content.as_bytes()).expect("write");
        let path = f.path().to_owned();
        (f, path)
    }

    #[test]
    fn deserialize_full_global_config() {
        // setup
        let toml = r#"
            [github]
            token = "gh-token"

            [bitbucket]
            username = "bob"
            app_password = "secret"

            [llm]
            default_provider = "gemini"
            fallback_provider = "codex"

            [preferences]
            context_lines = 10
            max_file_context = 1000
            large_pr_threshold = 3000
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert_eq!(
            raw.github.as_ref().unwrap().token.as_deref(),
            Some("gh-token")
        );
        assert_eq!(
            raw.bitbucket.as_ref().unwrap().username.as_deref(),
            Some("bob")
        );
        let llm = raw.llm.as_ref().unwrap();
        assert_eq!(llm.default_provider, Some(Provider::Gemini));
        assert_eq!(llm.fallback_provider, Some(Provider::Codex));
        let prefs = raw.preferences.as_ref().unwrap();
        assert_eq!(prefs.context_lines, Some(10));
    }

    #[test]
    fn deserialize_minimal_config() {
        // setup
        let toml = r#"
            [llm]
            default_provider = "claude"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert!(raw.github.is_none());
        assert!(raw.bitbucket.is_none());
        assert_eq!(
            raw.llm.as_ref().unwrap().default_provider,
            Some(Provider::Claude)
        );
        assert!(raw.preferences.is_none());
    }

    #[test]
    fn merge_repo_overrides_global() {
        // setup
        let global: RawGlobalConfig = toml::from_str(
            r#"
            [llm]
            default_provider = "claude"
            fallback_provider = "codex"
        "#,
        )
        .unwrap();

        let repo: RawRepoConfig = toml::from_str(
            r#"
            [llm]
            default_provider = "gemini"
        "#,
        )
        .unwrap();

        // execute
        let config = merge(Some(global), Some(repo));

        // verify — repo provider wins; fallback comes from global
        assert_eq!(config.llm.default_provider, Provider::Gemini);
        assert_eq!(config.llm.fallback_provider, Some(Provider::Codex));
    }

    #[test]
    fn merge_missing_repo_uses_global() {
        // setup
        let global: RawGlobalConfig = toml::from_str(
            r#"
            [llm]
            default_provider = "codex"

            [preferences]
            context_lines = 8
        "#,
        )
        .unwrap();

        // execute
        let config = merge(Some(global), None);

        // verify
        assert_eq!(config.llm.default_provider, Provider::Codex);
        assert_eq!(config.preferences.context_lines, 8);
    }

    #[test]
    fn merge_all_missing_uses_defaults() {
        // setup + execute
        let config = merge(None, None);

        // verify
        assert_eq!(config.llm.default_provider, Provider::Claude);
        assert!(config.llm.fallback_provider.is_none());
        assert_eq!(config.preferences.context_lines, DEFAULT_CONTEXT_LINES);
        assert_eq!(
            config.preferences.max_file_context,
            DEFAULT_MAX_FILE_CONTEXT
        );
        assert_eq!(
            config.preferences.large_pr_threshold,
            DEFAULT_LARGE_PR_THRESHOLD
        );
        assert!(config.github.token.is_none());
    }

    #[test]
    fn invalid_toml_returns_parse_error() {
        // setup
        let (_file, path) = write_temp("this is not valid toml!!!! [[[");

        // execute
        let result = parse_toml::<RawGlobalConfig>(&path);

        // verify
        assert!(
            matches!(result, Err(ConfigError::ParseError { .. })),
            "expected ParseError, got {result:?}"
        );
    }

    #[test]
    fn unknown_provider_returns_error() {
        // setup
        let toml = r#"
            [llm]
            default_provider = "gpt-4o"
        "#;

        // execute
        let result = toml::from_str::<RawGlobalConfig>(toml);

        // verify
        assert!(
            result.is_err(),
            "unknown provider should fail deserialization"
        );
    }

    #[test]
    fn deserialize_github_backend_gh() {
        // setup
        let toml = r#"
            [github]
            backend = "gh"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert_eq!(
            raw.github.as_ref().unwrap().backend,
            Some(GithubBackend::Gh)
        );
    }

    #[test]
    fn deserialize_github_backend_api() {
        // setup
        let toml = r#"
            [github]
            backend = "api"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert_eq!(
            raw.github.as_ref().unwrap().backend,
            Some(GithubBackend::Api)
        );
    }

    #[test]
    fn deserialize_github_backend_auto() {
        // setup
        let toml = r#"
            [github]
            backend = "auto"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert_eq!(
            raw.github.as_ref().unwrap().backend,
            Some(GithubBackend::Auto)
        );
    }

    #[test]
    fn deserialize_github_backend_missing_defaults_to_auto() {
        // setup
        let toml = r#"
            [github]
            token = "ghp_test"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        assert!(raw.github.as_ref().unwrap().backend.is_none());
        let config = merge(Some(raw), None);
        assert_eq!(config.github.backend, GithubBackend::Auto);
    }

    #[test]
    fn unknown_llm_field_rejected() {
        // setup
        let toml = r#"
            [llm]
            backend = "gemini"
        "#;

        // execute
        let result = toml::from_str::<RawGlobalConfig>(toml);

        // verify
        assert!(result.is_err(), "unknown field 'backend' should fail");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown field"),
            "error should say 'unknown field': {msg}"
        );
    }

    #[test]
    fn llm_provider_subtable_parsed() {
        // setup
        let toml = r#"
            [llm]
            default_provider = "gemini"

            [llm.gemini]
            command = "gemini"
            model = "gemini-3-pro-preview"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        let llm = raw.llm.as_ref().unwrap();
        assert_eq!(llm.default_provider, Some(Provider::Gemini));
        let gemini = llm.gemini.as_ref().unwrap();
        assert_eq!(gemini.command.as_deref(), Some("gemini"));
        assert_eq!(gemini.model.as_deref(), Some("gemini-3-pro-preview"));
    }

    #[test]
    fn llm_claude_code_alias_works() {
        // setup
        let toml = r#"
            [llm]
            default_provider = "claude"

            [llm.claude-code]
            command = "claude"
            model = "sonnet"
        "#;

        // execute
        let raw: RawGlobalConfig = toml::from_str(toml).expect("valid toml");

        // verify
        let llm = raw.llm.as_ref().unwrap();
        let claude = llm.claude.as_ref().unwrap();
        assert_eq!(claude.command.as_deref(), Some("claude"));
        assert_eq!(claude.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn llm_provider_settings_merged_into_config() {
        // setup
        let global: RawGlobalConfig = toml::from_str(
            r#"
            [llm]
            default_provider = "gemini"
            max_retries = 5

            [llm.gemini]
            model = "gemini-3-pro"
        "#,
        )
        .unwrap();

        // execute
        let config = merge(Some(global), None);

        // verify
        assert_eq!(config.llm.default_provider, Provider::Gemini);
        assert_eq!(config.llm.max_retries, 5);
        assert_eq!(config.llm.gemini.model.as_deref(), Some("gemini-3-pro"));
        assert!(config.llm.gemini.command.is_none());
    }

    #[test]
    fn deserialize_unknown_github_backend_fails() {
        // setup
        let toml = r#"
            [github]
            backend = "graphql"
        "#;

        // execute
        let result = toml::from_str::<RawGlobalConfig>(toml);

        // verify
        assert!(
            result.is_err(),
            "unknown backend should fail deserialization"
        );
    }
}
