//! GitHub REST API v3 client: PR listing and diff fetching.
//!
//! # Example
//! ```no_run
//! use easy_diff::platform::github::GithubClient;
//!
//! let client = GithubClient::new("ghp_token");
//! // let prs = client.list_open_pull_requests("owner", "repo").await?;
//! // let diff = client.get_pull_request_diff("owner", "repo", 42).await?;
//! ```

use serde::Deserialize;

use crate::platform::{PlatformError, PullRequest, PullRequestDiff};

const BASE_URL: &str = "https://api.github.com";

/// GitHub REST API v3 client authenticated with a personal access token.
pub struct GithubClient {
    client: reqwest::Client,
    token: String,
}

impl GithubClient {
    /// Creates a new client with the given personal access token.
    ///
    /// The underlying `reqwest::Client` is built with `User-Agent: easy-diff`
    /// (required by the GitHub API). An empty token is accepted here; the API
    /// call will fail with [`PlatformError::AuthError`] if the token is invalid.
    pub fn new(token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("easy-diff")
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            token: token.into(),
        }
    }

    /// Fetches open pull requests for the given repository.
    ///
    /// Returns up to 30 results (GitHub's default page size). Pagination is
    /// deferred to a future release.
    pub async fn list_open_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        let url = format!("{BASE_URL}/repos/{owner}/{repo}/pulls?state=open");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        check_rate_limit(&response);
        let response = check_status(response).await?;

        let github_prs: Vec<GithubPullRequest> = response.json().await?;
        Ok(github_prs.into_iter().map(PullRequest::from).collect())
    }

    /// Fetches metadata for a specific pull request, including base and head SHAs.
    ///
    /// Returns a [`PullRequest`] populated with all fields including SHA values
    /// required for cache key construction.
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, PlatformError> {
        let url = format!("{BASE_URL}/repos/{owner}/{repo}/pulls/{pr_number}");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        check_rate_limit(&response);
        let response = check_status(response).await?;

        let github_pr: GithubPullRequest = response.json().await?;
        Ok(PullRequest::from(github_pr))
    }

    /// Fetches the full unified diff for a specific pull request.
    ///
    /// The response body is the raw unified diff text, suitable for piping to
    /// `delta`, `bat`, or similar tools.
    pub async fn get_pull_request_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        let url = format!("{BASE_URL}/repos/{owner}/{repo}/pulls/{pr_number}");
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.diff")
            .send()
            .await?;

        check_rate_limit(&response);
        let response = check_status(response).await?;

        let diff = response.text().await?;
        Ok(PullRequestDiff { pr_number, diff })
    }
}

/// Reads `X-RateLimit-Remaining` and logs it at debug level.
fn check_rate_limit(response: &reqwest::Response) {
    if let Some(remaining) = response
        .headers()
        .get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok())
    {
        tracing::debug!(remaining, "GitHub rate limit remaining");
    }
}

/// Maps non-2xx responses to [`PlatformError`] variants.
///
/// - 401 → [`PlatformError::AuthError`]
/// - 403 + `X-RateLimit-Remaining: 0` → [`PlatformError::RateLimited`]
/// - 403 (other) → [`PlatformError::ApiError`]
/// - 404 → [`PlatformError::NotFound`]
/// - Other 4xx/5xx → [`PlatformError::ApiError`]
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, PlatformError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    // Read headers we need before consuming the response body.
    let remaining: Option<u64> = response
        .headers()
        .get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let reset_at = response
        .headers()
        .get("X-RateLimit-Reset")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let resource_path = response.url().path().to_string();

    match status.as_u16() {
        401 => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unauthorized".into());
            Err(PlatformError::AuthError { message })
        }
        403 if remaining == Some(0) => Err(PlatformError::RateLimited { reset_at }),
        404 => Err(PlatformError::NotFound {
            resource: resource_path,
        }),
        code => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            Err(PlatformError::ApiError {
                status: code,
                message,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Private GitHub API response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GithubPullRequest {
    number: u64,
    title: String,
    user: GithubUser,
    head: GithubRef,
    base: GithubRef,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct GithubUser {
    login: String,
}

#[derive(Deserialize)]
struct GithubRef {
    #[serde(rename = "ref")]
    ref_name: String,
    /// Git SHA of this ref's tip.
    sha: String,
}

impl From<GithubPullRequest> for PullRequest {
    fn from(pr: GithubPullRequest) -> Self {
        Self {
            number: pr.number,
            title: pr.title,
            author: pr.user.login,
            source_branch: pr.head.ref_name,
            target_branch: pr.base.ref_name,
            base_sha: pr.base.sha,
            head_sha: pr.head.sha,
            created_at: pr.created_at,
            updated_at: pr.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PR_LIST_JSON: &str = r#"[
        {
            "number": 42,
            "title": "Fix authentication bug",
            "user": { "login": "octocat" },
            "head": { "ref": "fix/auth-bug", "sha": "abc1234" },
            "base": { "ref": "main", "sha": "def5678" },
            "created_at": "2024-01-15T10:00:00Z",
            "updated_at": "2024-01-15T12:00:00Z",
            "labels": [],
            "milestone": null
        }
    ]"#;

    const SINGLE_PR_JSON: &str = r#"{
        "number": 42,
        "title": "Fix authentication bug",
        "user": { "login": "octocat" },
        "head": { "ref": "fix/auth-bug", "sha": "abc1234" },
        "base": { "ref": "main", "sha": "def5678" },
        "created_at": "2024-01-15T10:00:00Z",
        "updated_at": "2024-01-15T12:00:00Z"
    }"#;

    const PR_WITH_EXTRAS_JSON: &str = r#"{
        "number": 1,
        "title": "Test PR",
        "user": { "login": "user1" },
        "head": { "ref": "feature/test", "sha": "aaa0001" },
        "base": { "ref": "main", "sha": "bbb0002" },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "labels": [{"name": "bug"}],
        "milestone": {"title": "v1.0"},
        "body": "PR description",
        "state": "open",
        "draft": false
    }"#;

    #[test]
    fn deserialize_pr_list_response() {
        // setup
        // execute
        let prs: Vec<GithubPullRequest> = serde_json::from_str(PR_LIST_JSON).unwrap();

        // verify
        assert_eq!(prs.len(), 1);
        let pr = &prs[0];
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix authentication bug");
        assert_eq!(pr.user.login, "octocat");
        assert_eq!(pr.head.ref_name, "fix/auth-bug");
        assert_eq!(pr.head.sha, "abc1234");
        assert_eq!(pr.base.ref_name, "main");
        assert_eq!(pr.base.sha, "def5678");
        assert_eq!(pr.created_at, "2024-01-15T10:00:00Z");
        assert_eq!(pr.updated_at, "2024-01-15T12:00:00Z");
    }

    #[test]
    fn convert_github_pr_to_pull_request() {
        // setup
        let github_pr: GithubPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // execute
        let pr = PullRequest::from(github_pr);

        // verify
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix authentication bug");
        assert_eq!(pr.author, "octocat");
        assert_eq!(pr.source_branch, "fix/auth-bug");
        assert_eq!(pr.target_branch, "main");
        assert_eq!(pr.head_sha, "abc1234");
        assert_eq!(pr.base_sha, "def5678");
        assert_eq!(pr.created_at, "2024-01-15T10:00:00Z");
        assert_eq!(pr.updated_at, "2024-01-15T12:00:00Z");
    }

    #[test]
    fn deserialize_single_pr() {
        // setup
        // execute
        let pr: GithubPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // verify
        assert_eq!(pr.number, 42);
        assert_eq!(pr.user.login, "octocat");
        assert_eq!(pr.head.ref_name, "fix/auth-bug");
        assert_eq!(pr.head.sha, "abc1234");
    }

    #[test]
    fn deserialize_ignores_extra_fields() {
        // setup
        // execute
        let pr: GithubPullRequest = serde_json::from_str(PR_WITH_EXTRAS_JSON).unwrap();

        // verify — extra fields (labels, milestone, body, state, draft) are silently ignored
        assert_eq!(pr.number, 1);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.user.login, "user1");
    }
}
