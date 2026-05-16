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

use crate::analysis::{AnalysisEngine, AnalysisResult, PrContext};
use crate::cache::{CacheKey, CacheStore, CategoryOverride, CommentStatus, ReviewComment};
use crate::config::{self, Provider};
use crate::diff::{parse_diff, DiffFile};
use crate::git;
use crate::llm::schema::Pass1Output;
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
    let client = github_provider::create_github_provider(&config.github, &repo_info.host)
        .await
        .map_err(|e| e.to_string())?;
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

/// Runs the two-pass analysis for the given PR and returns the combined result.
///
/// Fetches the diff via the platform API, then runs the two-pass LLM analysis
/// engine. Because [`AnalysisEngine`] is `!Send` (it wraps `rusqlite::Connection`
/// which uses `RefCell`), the engine is constructed and driven on a dedicated
/// single-threaded tokio runtime inside [`tokio::task::spawn_blocking`]. The
/// result is sent back to the caller via a one-shot channel.
#[tauri::command]
async fn run_analysis(pr_number: u64) -> Result<AnalysisResult, String> {
    use std::sync::Arc;

    // Fetch PR metadata and diff on the current async context (Send-safe).
    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let config = config::load_config(Some(&repo_info.root)).map_err(|e| e.to_string())?;
    let client = github_provider::create_github_provider(&config.github, &repo_info.host)
        .await
        .map_err(|e| e.to_string())?;

    let (pr, diff_data) = tokio::try_join!(
        client.get_pull_request(&repo_info.owner, &repo_info.repo, pr_number),
        client.get_pull_request_diff(&repo_info.owner, &repo_info.repo, pr_number),
    )
    .map_err(|e| e.to_string())?;

    // Move all non-Send work (AnalysisEngine) to a dedicated thread with its own runtime.
    let prefs = config.preferences.clone();
    let root = repo_info.root.clone();
    let dispatcher = Arc::new(crate::llm::create_dispatcher(&config));
    let diff_text = diff_data.diff;
    let base_sha = pr.base_sha;
    let head_sha = pr.head_sha;

    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        rt.block_on(async move {
            let engine = AnalysisEngine::new(
                dispatcher,
                prefs.large_pr_threshold,
                5,
                prefs.max_file_context,
                None, // no cache — GUI returns result directly
                false,
                root,
            );
            let ctx = PrContext {
                pr_number,
                base_sha,
                head_sha,
            };
            engine
                .run(&diff_text, &ctx)
                .await
                .map_err(|e| e.to_string())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns a cached analysis result for the given PR, reading directly from SQLite.
///
/// Cache reads are synchronous (rusqlite), so they run in `spawn_blocking`.
/// Returns `None` when no analysis has been cached yet.
#[tauri::command]
async fn get_analysis(pr_number: u64) -> Result<Option<AnalysisResult>, String> {
    use std::sync::Arc;

    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let config = config::load_config(Some(&repo_info.root)).map_err(|e| e.to_string())?;
    let client = github_provider::create_github_provider(&config.github, &repo_info.host)
        .await
        .map_err(|e| e.to_string())?;

    let pr = client
        .get_pull_request(&repo_info.owner, &repo_info.repo, pr_number)
        .await
        .map_err(|e| e.to_string())?;

    let provider_name = {
        let dispatcher = Arc::new(crate::llm::create_dispatcher(&config));
        dispatcher.provider_name().to_owned()
    };

    let db_path = repo_info
        .root
        .join(".easy-diff")
        .join("cache")
        .join("analysis.db");
    let key = CacheKey {
        pr_id: pr_number.to_string(),
        base_sha: pr.base_sha,
        head_sha: pr.head_sha,
        provider: provider_name,
    };

    tokio::task::spawn_blocking(move || -> Result<Option<AnalysisResult>, String> {
        let cache = match CacheStore::open(&db_path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let pass1 = match cache.get_pass1(&key).map_err(|e| e.to_string())? {
            Some(p1) => p1,
            None => return Ok(None),
        };

        let cached_files = cache.get_cached_files(&key).map_err(|e| e.to_string())?;
        let mut files = std::collections::HashMap::new();
        for path in cached_files {
            if let Ok(Some(p2)) = cache.get_pass2(&key, &path) {
                files.insert(path, p2);
            }
        }

        Ok(Some(AnalysisResult {
            pass1,
            files,
            has_changes_since_viewed: std::collections::HashMap::new(),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns the parsed diff for the given PR as a structured list of [`DiffFile`]s.
#[tauri::command]
async fn get_diff(pr_number: u64) -> Result<Vec<DiffFile>, String> {
    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let config = config::load_config(Some(&repo_info.root)).map_err(|e| e.to_string())?;
    let client = github_provider::create_github_provider(&config.github, &repo_info.host)
        .await
        .map_err(|e| e.to_string())?;

    let diff_data = client
        .get_pull_request_diff(&repo_info.owner, &repo_info.repo, pr_number)
        .await
        .map_err(|e| e.to_string())?;

    Ok(parse_diff(&diff_data.diff))
}

/// Returns the cached Pass 1 summary for the given PR, if available.
///
/// Cache reads are synchronous (rusqlite), so they run in `spawn_blocking`.
#[tauri::command]
async fn get_pass1_summary(pr_number: u64) -> Result<Option<Pass1Output>, String> {
    use std::sync::Arc;

    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let config = config::load_config(Some(&repo_info.root)).map_err(|e| e.to_string())?;
    let client = github_provider::create_github_provider(&config.github, &repo_info.host)
        .await
        .map_err(|e| e.to_string())?;

    let pr = client
        .get_pull_request(&repo_info.owner, &repo_info.repo, pr_number)
        .await
        .map_err(|e| e.to_string())?;

    let provider_name = {
        let dispatcher = Arc::new(crate::llm::create_dispatcher(&config));
        dispatcher.provider_name().to_owned()
    };

    let db_path = repo_info
        .root
        .join(".easy-diff")
        .join("cache")
        .join("analysis.db");
    let key = CacheKey {
        pr_id: pr_number.to_string(),
        base_sha: pr.base_sha,
        head_sha: pr.head_sha,
        provider: provider_name,
    };

    tokio::task::spawn_blocking(move || -> Result<Option<Pass1Output>, String> {
        let cache = match CacheStore::open(&db_path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        cache.get_pass1(&key).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Review comment Tauri commands
// ---------------------------------------------------------------------------

/// Opens the SQLite cache at the canonical path for the current repository.
///
/// Returns an error string if the repository root cannot be detected or the
/// database cannot be opened.
fn open_cache() -> Result<CacheStore, String> {
    let repo_info = git::detect_repo_info(".").map_err(|e| e.to_string())?;
    let db_path = repo_info
        .root
        .join(".easy-diff")
        .join("cache")
        .join("analysis.db");
    CacheStore::open(&db_path).map_err(|e| e.to_string())
}

/// Adds a new draft review comment on a line (or line range) within a file.
///
/// Returns the persisted [`ReviewComment`] with its database-assigned `id`.
#[tauri::command]
fn add_comment(
    pr_id: String,
    file_path: String,
    start_line: u32,
    end_line: Option<u32>,
    body: String,
) -> Result<ReviewComment, String> {
    let cache = open_cache()?;
    let now = {
        // Minimal ISO 8601 UTC timestamp without pulling in chrono.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}Z", secs)
    };
    let comment = ReviewComment {
        id: 0,
        pr_id,
        file_path,
        start_line,
        end_line,
        body,
        created_at: now,
        status: CommentStatus::Draft,
    };
    cache.add_comment(&comment).map_err(|e| e.to_string())
}

/// Returns all review comments for the given PR.
#[tauri::command]
fn list_comments(pr_id: String) -> Result<Vec<ReviewComment>, String> {
    let cache = open_cache()?;
    cache.list_comments(&pr_id).map_err(|e| e.to_string())
}

/// Updates the body of an existing comment (keeps status as Draft).
#[tauri::command]
fn update_comment(id: i64, body: String) -> Result<(), String> {
    let cache = open_cache()?;
    cache
        .update_comment(id, &body, &CommentStatus::Draft)
        .map_err(|e| e.to_string())
        .map(|_| ())
}

/// Deletes a comment by id.
#[tauri::command]
fn delete_comment(id: i64) -> Result<(), String> {
    let cache = open_cache()?;
    cache
        .delete_comment(id)
        .map_err(|e| e.to_string())
        .map(|_| ())
}

/// Sets (or replaces) a category override for a specific hunk.
///
/// Pass `change_type: null` / `attention_tags: null` to clear the respective override.
#[tauri::command]
fn set_category_override(
    pr_id: String,
    file_path: String,
    hunk_id: String,
    change_type: Option<String>,
    attention_tags: Option<Vec<String>>,
) -> Result<(), String> {
    let cache = open_cache()?;
    cache
        .set_override(
            &pr_id,
            &file_path,
            &hunk_id,
            change_type.as_deref(),
            attention_tags.as_deref(),
        )
        .map_err(|e| e.to_string())
        .map(|_| ())
}

/// Returns all category overrides for the given PR.
#[tauri::command]
fn get_category_overrides(pr_id: String) -> Result<Vec<CategoryOverride>, String> {
    let cache = open_cache()?;
    cache.get_overrides(&pr_id).map_err(|e| e.to_string())
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
            run_analysis,
            get_analysis,
            get_diff,
            get_pass1_summary,
            add_comment,
            list_comments,
            update_comment,
            delete_comment,
            set_category_override,
            get_category_overrides,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
