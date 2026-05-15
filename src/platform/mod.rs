//! Platform API clients for GitHub and BitBucket Cloud.
//! Responsible for PR listing, diff fetching, and (future) review submission.
//!
//! The [`Platform`] enum identifies the hosting service from a remote URL.
//! [`PullRequest`] and [`PullRequestDiff`] are platform-agnostic types returned
//! by all clients. [`PlatformError`] covers the full range of API failure modes.

#![allow(dead_code)]

pub mod bitbucket;
pub mod github;

/// Supported code hosting platforms, identified from remote URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct PullRequest {
    /// PR number (unique within a repository).
    pub number: u64,
    /// PR title.
    pub title: String,
    /// Author username.
    pub author: String,
    /// Source (head) branch name.
    pub source_branch: String,
    /// Target (base) branch name.
    pub target_branch: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

/// The raw unified diff for a pull request.
#[derive(Debug, Clone)]
pub struct PullRequestDiff {
    /// PR number this diff belongs to.
    pub pr_number: u64,
    /// Full unified diff as a string.
    pub diff: String,
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
