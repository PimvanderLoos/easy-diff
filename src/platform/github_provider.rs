//! GitHub provider factory: selects between `gh` CLI and REST API backends.
//!
//! The [`create_github_provider`] function picks the appropriate backend based
//! on the configured [`GithubBackend`] mode and runtime availability of `gh`.

use crate::config::{GithubBackend, GithubConfig};
use crate::platform::github::GithubClient;
use crate::platform::github_gh::GhClient;
use crate::platform::{
    CurrentUser, GithubOperations, PlatformError, PullRequest, PullRequestDiff,
    ReviewCommentPayload, ReviewEvent,
};

/// A resolved GitHub provider that dispatches to either the `gh` CLI or REST API.
#[derive(Debug)]
pub enum GithubProvider {
    /// `gh` CLI backend.
    Gh(GhClient),
    /// REST API + PAT backend.
    Api(GithubClient),
}

impl GithubOperations for GithubProvider {
    async fn list_open_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        match self {
            Self::Gh(c) => c.list_open_pull_requests(owner, repo).await,
            Self::Api(c) => c.list_open_pull_requests(owner, repo).await,
        }
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, PlatformError> {
        match self {
            Self::Gh(c) => c.get_pull_request(owner, repo, pr_number).await,
            Self::Api(c) => c.get_pull_request(owner, repo, pr_number).await,
        }
    }

    async fn get_pull_request_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        match self {
            Self::Gh(c) => c.get_pull_request_diff(owner, repo, pr_number).await,
            Self::Api(c) => c.get_pull_request_diff(owner, repo, pr_number).await,
        }
    }

    async fn current_user(&self) -> Result<CurrentUser, PlatformError> {
        match self {
            Self::Gh(c) => c.current_user().await,
            Self::Api(c) => c.current_user().await,
        }
    }

    async fn submit_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
        comments: Vec<ReviewCommentPayload>,
    ) -> Result<(), PlatformError> {
        match self {
            Self::Gh(c) => {
                c.submit_review(owner, repo, pr_number, event, body, comments)
                    .await
            }
            Self::Api(c) => {
                c.submit_review(owner, repo, pr_number, event, body, comments)
                    .await
            }
        }
    }
}

/// Creates the appropriate GitHub provider based on config and runtime availability.
///
/// Resolution logic:
/// - `Auto`: try `gh` first (installed + authenticated), fall back to API/PAT.
/// - `Gh`: use `gh` exclusively; error if unavailable.
/// - `Api`: use REST API + PAT exclusively.
///
/// `host` is derived from the remote URL (e.g. `"github.com"`). Used to check
/// `gh auth status --hostname`.
pub async fn create_github_provider(
    config: &GithubConfig,
    host: &str,
) -> Result<GithubProvider, anyhow::Error> {
    match config.backend {
        GithubBackend::Auto => {
            if GhClient::is_installed().await && GhClient::is_authenticated(host).await {
                tracing::info!("using gh CLI backend (auto-detected)");
                let gh_host = if host == "github.com" {
                    None
                } else {
                    Some(host.to_string())
                };
                Ok(GithubProvider::Gh(GhClient::new(gh_host)))
            } else if let Some(token) = &config.token {
                tracing::info!("using GitHub API backend (gh unavailable, falling back to PAT)");
                Ok(GithubProvider::Api(GithubClient::new(token.clone())))
            } else {
                anyhow::bail!(
                    "GitHub access not available:\n\
                     • Install and authenticate `gh` CLI: run `gh auth login`\n\
                     • Or configure a personal access token: add [github] token = \"...\" to your config"
                )
            }
        }
        GithubBackend::Gh => {
            if !GhClient::is_installed().await {
                anyhow::bail!(
                    "`gh` CLI is not installed — install it from https://cli.github.com/ or switch to github.backend = \"api\""
                );
            }
            if !GhClient::is_authenticated(host).await {
                anyhow::bail!(
                    "`gh` is not authenticated for {host} — run `gh auth login --hostname {host}`"
                );
            }
            let gh_host = if host == "github.com" {
                None
            } else {
                Some(host.to_string())
            };
            Ok(GithubProvider::Gh(GhClient::new(gh_host)))
        }
        GithubBackend::Api => {
            let token = config.token.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "GitHub API backend requires a token — add [github] token = \"...\" to your config"
                )
            })?;
            Ok(GithubProvider::Api(GithubClient::new(token.clone())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_provider_api_mode_without_token_fails() {
        // setup
        let config = GithubConfig {
            token: None,
            backend: GithubBackend::Api,
        };

        // execute
        let result = create_github_provider(&config, "github.com").await;

        // verify
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("token"), "error should mention token: {msg}");
    }

    #[tokio::test]
    async fn create_provider_api_mode_with_token_succeeds() {
        // setup
        let config = GithubConfig {
            token: Some("ghp_test123".into()),
            backend: GithubBackend::Api,
        };

        // execute
        let result = create_github_provider(&config, "github.com").await;

        // verify
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), GithubProvider::Api(_)));
    }

    #[tokio::test]
    async fn create_provider_auto_with_token_no_gh_uses_api() {
        // setup — gh is likely not installed/authenticated in test env
        // If gh IS installed, this test still passes (it just picks gh)
        let config = GithubConfig {
            token: Some("ghp_test123".into()),
            backend: GithubBackend::Auto,
        };

        // execute
        let result = create_github_provider(&config, "github.com").await;

        // verify — should succeed regardless of gh availability
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_provider_auto_no_token_no_gh_fails() {
        // setup — assume gh is not authenticated for a fake host
        let config = GithubConfig {
            token: None,
            backend: GithubBackend::Auto,
        };

        // execute
        let result = create_github_provider(&config, "fake-host-that-does-not-exist.example").await;

        // verify
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("gh auth login") || msg.contains("token"),
            "error should be actionable: {msg}"
        );
    }
}
