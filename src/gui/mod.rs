//! Tauri GUI entry point, compiled only when the `gui` feature is enabled.
//!
//! # Example
//!
//! ```no_run
//! #[cfg(feature = "gui")]
//! easy_diff::gui::run();
//! ```
//!
//! This module owns the `tauri::Builder` setup and exposes a `run()` function
//! that `main.rs` calls when the binary is launched in GUI mode.  Tauri
//! command handlers are registered here and callable from the frontend via
//! `invoke()`.

use serde::Serialize;

use crate::config::{self, Provider};
use crate::git;
use crate::platform::github_provider;
use crate::platform::{GithubOperations, PullRequest};

// ---------------------------------------------------------------------------
// Serialisable DTOs for Tauri IPC
// ---------------------------------------------------------------------------

/// Subset of [`crate::config::Config`] exposed to the frontend.
///
/// Only the fields needed by the GUI are included; secrets (tokens, passwords)
/// are intentionally omitted.
#[derive(Debug, Serialize)]
pub struct GuiConfig {
    /// Active LLM provider name.
    pub default_provider: Provider,
    /// Fallback LLM provider, if configured.
    pub fallback_provider: Option<Provider>,
    /// GitHub backend in use (`auto`, `gh`, or `api`).
    pub github_backend: String,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Lists open pull requests for the repository in the current working directory.
///
/// Returns a serialised [`Vec<PullRequest>`] on success, or a human-readable
/// error string on failure (Tauri IPC requires `String` errors).
#[tauri::command]
async fn list_pull_requests() -> Result<Vec<PullRequest>, String> {
    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let config = config::load_config(Some(&repo_info.root)).map_err(|e| e.to_string())?;
    let client = github_provider::create_github_provider(&config.github);
    client
        .list_open_pull_requests(&repo_info.owner, &repo_info.repo)
        .await
        .map_err(|e| e.to_string())
}

/// Returns repository information for the current working directory.
///
/// Wraps [`crate::git::detect_repo_info`].
#[tauri::command]
async fn get_repo_info() -> Result<git::RepoInfo, String> {
    git::detect_repo_info(".").map_err(|e| e.to_string())
}

/// Returns the resolved configuration, omitting secrets.
///
/// Wraps [`crate::config::load_config`] and maps to [`GuiConfig`].
#[tauri::command]
async fn get_config() -> Result<GuiConfig, String> {
    let config = config::load_config(None).map_err(|e| e.to_string())?;
    Ok(GuiConfig {
        default_provider: config.llm.default_provider,
        fallback_provider: config.llm.fallback_provider,
        github_backend: format!("{:?}", config.github.backend).to_lowercase(),
    })
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

/// Launch the Tauri GUI application.
///
/// # Panics
///
/// Panics if Tauri cannot initialise the application (e.g. missing WebView
/// runtime).  This mirrors the idiomatic Tauri pattern of calling `.expect()`
/// on the final `run()` call.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_pull_requests,
            get_repo_info,
            get_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
