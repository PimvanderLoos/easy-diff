//! Platform API clients for GitHub and BitBucket Cloud.
//! Responsible for PR listing, diff fetching, and review submission.
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
//!
//! Review submission uses [`ReviewEvent`] and [`ReviewCommentPayload`] to build the
//! payload for `GithubOperations::submit_review`.

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

/// Review verdict sent to the GitHub API.
///
/// Serialises to the uppercase strings required by the GitHub Reviews API
/// (`APPROVE`, `REQUEST_CHANGES`, `COMMENT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    /// Approve the pull request.
    Approve,
    /// Request changes before the PR can be merged.
    RequestChanges,
    /// Leave a neutral comment review (no approval or rejection).
    Comment,
}

/// A single inline comment to attach to a review.
///
/// Used by [`GithubOperations::submit_review`] to post line-level feedback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewCommentPayload {
    /// Repository-relative file path.
    pub path: String,
    /// Line number in the new file (1-based) at which to anchor the comment.
    pub line: u32,
    /// Markdown body of the comment.
    pub body: String,
}

/// Trait for GitHub operations (list PRs, get metadata, get diff, submit review).
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

    /// Submits a review (with optional inline comments) to a pull request.
    ///
    /// `event` controls whether the review approves, requests changes, or is a
    /// neutral comment. `body` is the top-level review text (optional).
    /// `comments` are the inline line-level comments to attach.
    async fn submit_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
        comments: Vec<ReviewCommentPayload>,
    ) -> Result<(), PlatformError>;
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
