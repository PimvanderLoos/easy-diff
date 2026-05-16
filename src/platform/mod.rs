//! Platform API clients for GitHub and BitBucket Cloud.
//! Responsible for PR listing, diff fetching, and (future) review submission.
//!
//! The [`Platform`] enum identifies the hosting service from a remote URL.
//! [`PullRequest`] and [`PullRequestDiff`] are platform-agnostic types returned
//! by all clients. [`PlatformError`] covers the full range of API failure modes.
//!
//! For GitHub, two backends are available:
//! - [`github::GithubClient`] — REST API with personal access token
//! - [`github_gh::GhClient`] — `gh` CLI wrapper using `gh auth login` credentials
//!
//! Use [`github_provider::create_github_provider`] to select the appropriate backend.

pub mod bitbucket;
pub mod github;
pub mod github_gh;
pub mod github_provider;

/// Supported code hosting platforms, identified from remote URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Platform {
    GitHub,
    BitBucket,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "GitHub"),
            Self::BitBucket => write!(f, "BitBucket"),
        }
    }
}

/// A pull request from any supported platform.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PullRequest {
    /// PR number (unique within a repository).
    pub number: u64,
    /// PR title.
    pub title: String,
    /// Author username.
    pub author: String,
    /// Source (head) branch name.
    #[allow(dead_code)]
    pub source_branch: String,
    /// Target (base) branch name.
    #[allow(dead_code)]
    pub target_branch: String,
    /// Git SHA of the base (target) branch tip at PR creation time.
    pub base_sha: String,
    /// Git SHA of the head (source) branch tip.
    pub head_sha: String,
    /// ISO 8601 creation timestamp.
    #[allow(dead_code)]
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

/// The raw unified diff for a pull request.
#[derive(Debug, Clone)]
pub struct PullRequestDiff {
    /// PR number this diff belongs to.
    #[allow(dead_code)]
    pub pr_number: u64,
    /// Full unified diff as a string.
    pub diff: String,
}

/// Trait for GitHub operations (list PRs, get metadata, get diff).
///
/// Implemented by both the REST API client and the `gh` CLI client.
#[allow(async_fn_in_trait)]
pub trait GithubOperations {
    /// Lists open pull requests for the given repository.
    async fn list_open_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError>;

    /// Fetches metadata for a specific pull request.
    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, PlatformError>;

    /// Fetches the unified diff for a specific pull request.
    async fn get_pull_request_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError>;
}

/// Errors from platform API calls.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("authentication failed: {message}")]
    AuthError { message: String },
    #[error("{resource} not found")]
    NotFound { resource: String },
    #[error("rate limited (resets at {reset_at})")]
    RateLimited { reset_at: String },
    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}
