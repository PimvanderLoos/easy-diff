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

/// The authenticated user on the hosting platform (i.e. the reviewer).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrentUser {
    /// Login / username on the platform.
    pub login: String,
    /// URL of the user's avatar image, when the platform exposes one.
    pub avatar_url: Option<String>,
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(async_fn_in_trait, dead_code)]
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

    /// Fetches the authenticated user (the reviewer) for this client.
    async fn current_user(&self) -> Result<CurrentUser, PlatformError>;

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

/// Trait for BitBucket Cloud operations (list PRs, get metadata, get diff).
///
/// Implemented by [`bitbucket::BitbucketClient`].
#[allow(async_fn_in_trait, dead_code)]
pub trait BitbucketOperations {
    /// Lists open pull requests for the given repository.
    async fn list_open_pull_requests(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<PullRequest>, PlatformError>;

    /// Fetches metadata for a specific pull request.
    async fn get_pull_request(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequest, PlatformError>;

    /// Fetches the unified diff for a specific pull request.
    async fn get_pull_request_diff(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequestDiff, PlatformError>;

    /// Returns the authenticated user (the reviewer) for this client.
    async fn current_user(&self) -> Result<CurrentUser, PlatformError>;
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
    #[allow(dead_code)]
    #[error("operation not supported on {platform}: {operation}")]
    Unsupported { platform: String, operation: String },
}

// ---------------------------------------------------------------------------
// Platform-agnostic client
// ---------------------------------------------------------------------------

/// Unified platform client that dispatches to the appropriate backend.
///
/// Created via [`create_platform_client`] based on the detected platform.
#[derive(Debug)]
pub enum PlatformClient {
    /// GitHub (REST API or gh CLI).
    Github(github_provider::GithubProvider),
    /// BitBucket Cloud (REST API).
    Bitbucket(bitbucket::BitbucketClient),
}

impl PlatformClient {
    /// Lists open pull requests for the repository.
    pub async fn list_open_pull_requests(
        &self,
        owner_or_workspace: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        match self {
            Self::Github(c) => c.list_open_pull_requests(owner_or_workspace, repo).await,
            Self::Bitbucket(c) => c.list_open_pull_requests(owner_or_workspace, repo).await,
        }
    }

    /// Fetches metadata for a specific pull request.
    pub async fn get_pull_request(
        &self,
        owner_or_workspace: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, PlatformError> {
        match self {
            Self::Github(c) => {
                c.get_pull_request(owner_or_workspace, repo, pr_number)
                    .await
            }
            Self::Bitbucket(c) => {
                c.get_pull_request(owner_or_workspace, repo, pr_number)
                    .await
            }
        }
    }

    /// Fetches the unified diff for a specific pull request.
    pub async fn get_pull_request_diff(
        &self,
        owner_or_workspace: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        match self {
            Self::Github(c) => {
                c.get_pull_request_diff(owner_or_workspace, repo, pr_number)
                    .await
            }
            Self::Bitbucket(c) => {
                c.get_pull_request_diff(owner_or_workspace, repo, pr_number)
                    .await
            }
        }
    }

    /// Returns the authenticated user (the reviewer) for this client.
    pub async fn current_user(&self) -> Result<CurrentUser, PlatformError> {
        match self {
            Self::Github(c) => c.current_user().await,
            Self::Bitbucket(c) => c.current_user().await,
        }
    }

    /// Submits a review. Only supported on GitHub; returns an error for BitBucket.
    #[allow(dead_code)]
    pub async fn submit_review(
        &self,
        owner_or_workspace: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
        comments: Vec<ReviewCommentPayload>,
    ) -> Result<(), PlatformError> {
        match self {
            Self::Github(c) => {
                c.submit_review(owner_or_workspace, repo, pr_number, event, body, comments)
                    .await
            }
            Self::Bitbucket(_) => Err(PlatformError::Unsupported {
                platform: "BitBucket".into(),
                operation: "submit_review".into(),
            }),
        }
    }
}

/// Creates a platform client based on the detected platform and configuration.
///
/// - `Platform::GitHub` → resolves via [`github_provider::create_github_provider`]
/// - `Platform::BitBucket` → creates a [`bitbucket::BitbucketClient`] from config credentials
pub async fn create_platform_client(
    platform: Platform,
    config: &crate::config::Config,
    host: &str,
) -> Result<PlatformClient, anyhow::Error> {
    match platform {
        Platform::GitHub => {
            let provider = github_provider::create_github_provider(&config.github, host).await?;
            Ok(PlatformClient::Github(provider))
        }
        Platform::BitBucket => {
            let username = config.bitbucket.username.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "BitBucket access requires credentials — add [bitbucket] username = \"...\" and app_password = \"...\" to your config"
                )
            })?;
            let app_password = config.bitbucket.app_password.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "BitBucket access requires an app password — add [bitbucket] app_password = \"...\" to your config"
                )
            })?;
            Ok(PlatformClient::Bitbucket(bitbucket::BitbucketClient::new(
                username.clone(),
                app_password.clone(),
            )))
        }
    }
}
