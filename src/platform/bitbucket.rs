//! BitBucket Cloud REST API 2.0 client: PR listing and diff fetching.
//!
//! # Example
//! ```no_run
//! use easy_diff::platform::bitbucket::BitbucketClient;
//!
//! let client = BitbucketClient::new("username", "app_password");
//! // let prs = client.list_open_pull_requests("workspace", "repo").await?;
//! // let diff = client.get_pull_request_diff("workspace", "repo", 1).await?;
//! ```

use serde::Deserialize;

use crate::platform::{
    BitbucketOperations, CurrentUser, PlatformError, PullRequest, PullRequestDiff,
};

const BASE_URL: &str = "https://api.bitbucket.org/2.0";

/// BitBucket Cloud REST API 2.0 client authenticated with an app password.
#[derive(Debug)]
#[allow(dead_code)]
pub struct BitbucketClient {
    client: reqwest::Client,
    username: String,
    app_password: String,
}

#[allow(dead_code)]
impl BitbucketClient {
    /// Creates a new client with the given username and app password.
    ///
    /// BitBucket Cloud uses HTTP Basic authentication with username + app password
    /// (not the account password). Empty credentials are accepted; the API call
    /// will fail with [`PlatformError::AuthError`] if they are invalid.
    pub fn new(username: impl Into<String>, app_password: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("easy-diff")
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            username: username.into(),
            app_password: app_password.into(),
        }
    }

    /// Fetches open pull requests for the given repository.
    ///
    /// Returns up to 50 results (BitBucket's default page size). Pagination is
    /// deferred to a future release.
    pub async fn list_open_pull_requests(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        let url =
            format!("{BASE_URL}/repositories/{workspace}/{repo_slug}/pullrequests?state=OPEN");
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .send()
            .await?;

        let response = check_status(response).await?;
        let paginated: BitbucketPaginatedResponse = response.json().await?;
        Ok(paginated
            .values
            .into_iter()
            .map(PullRequest::from)
            .collect())
    }

    /// Fetches metadata for a specific pull request.
    pub async fn get_pull_request(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequest, PlatformError> {
        let url = format!("{BASE_URL}/repositories/{workspace}/{repo_slug}/pullrequests/{pr_id}");
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .send()
            .await?;

        let response = check_status(response).await?;
        let bb_pr: BitbucketPullRequest = response.json().await?;
        Ok(PullRequest::from(bb_pr))
    }

    /// Fetches the unified diff for a specific pull request.
    pub async fn get_pull_request_diff(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        let url =
            format!("{BASE_URL}/repositories/{workspace}/{repo_slug}/pullrequests/{pr_id}/diff");
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .send()
            .await?;

        let response = check_status(response).await?;
        let diff = response.text().await?;
        Ok(PullRequestDiff {
            pr_number: pr_id,
            diff,
        })
    }
}

impl BitbucketOperations for BitbucketClient {
    async fn list_open_pull_requests(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        self.list_open_pull_requests(workspace, repo_slug).await
    }

    async fn get_pull_request(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequest, PlatformError> {
        self.get_pull_request(workspace, repo_slug, pr_id).await
    }

    async fn get_pull_request_diff(
        &self,
        workspace: &str,
        repo_slug: &str,
        pr_id: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        self.get_pull_request_diff(workspace, repo_slug, pr_id)
            .await
    }

    async fn current_user(&self) -> Result<CurrentUser, PlatformError> {
        // BitBucket Cloud authenticates with the configured username; the REST
        // API does not surface an avatar URL through this credential flow.
        Ok(CurrentUser {
            login: self.username.clone(),
            avatar_url: None,
        })
    }
}

/// Maps non-2xx responses to [`PlatformError`] variants.
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, PlatformError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let resource_path = response.url().path().to_string();

    match status.as_u16() {
        401 => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "unauthorized".into());
            Err(PlatformError::AuthError { message })
        }
        404 => Err(PlatformError::NotFound {
            resource: resource_path,
        }),
        429 => Err(PlatformError::RateLimited {
            reset_at: "unknown".into(),
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
// Private BitBucket API response types
// ---------------------------------------------------------------------------

/// Paginated response wrapper from BitBucket's REST API.
#[derive(Deserialize)]
struct BitbucketPaginatedResponse {
    values: Vec<BitbucketPullRequest>,
}

/// A pull request as returned by the BitBucket API.
#[derive(Deserialize)]
struct BitbucketPullRequest {
    id: u64,
    title: String,
    author: BitbucketAuthor,
    source: BitbucketEndpoint,
    destination: BitbucketEndpoint,
    created_on: String,
    updated_on: String,
}

/// Author object from BitBucket PR response.
#[derive(Deserialize)]
struct BitbucketAuthor {
    /// Account nickname (short username).
    nickname: Option<String>,
    /// Full display name (fallback when nickname is absent).
    display_name: String,
}

/// Source or destination endpoint of a BitBucket PR.
#[derive(Deserialize)]
struct BitbucketEndpoint {
    branch: BitbucketBranch,
    commit: BitbucketCommit,
}

/// Branch reference within a BitBucket endpoint.
#[derive(Deserialize)]
struct BitbucketBranch {
    name: String,
}

/// Commit reference within a BitBucket endpoint.
#[derive(Deserialize)]
struct BitbucketCommit {
    hash: String,
}

impl From<BitbucketPullRequest> for PullRequest {
    fn from(pr: BitbucketPullRequest) -> Self {
        Self {
            number: pr.id,
            title: pr.title,
            author: pr.author.nickname.unwrap_or(pr.author.display_name),
            source_branch: pr.source.branch.name,
            target_branch: pr.destination.branch.name,
            base_sha: pr.destination.commit.hash,
            head_sha: pr.source.commit.hash,
            created_at: pr.created_on,
            updated_at: pr.updated_on,
            // BitBucket's PR list endpoint doesn't expose change stats.
            changed_files: None,
            additions: None,
            deletions: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PR_LIST_JSON: &str = r#"{
        "values": [
            {
                "id": 1,
                "title": "Add feature X",
                "author": {
                    "display_name": "Alice Smith",
                    "nickname": "asmith"
                },
                "source": {
                    "branch": { "name": "feature/x" },
                    "commit": { "hash": "abc1234567890" }
                },
                "destination": {
                    "branch": { "name": "main" },
                    "commit": { "hash": "def0987654321" }
                },
                "created_on": "2024-03-01T10:00:00.000000+00:00",
                "updated_on": "2024-03-01T12:30:00.000000+00:00"
            },
            {
                "id": 2,
                "title": "Fix bug Y",
                "author": {
                    "display_name": "Bob Jones",
                    "nickname": "bjones"
                },
                "source": {
                    "branch": { "name": "fix/bug-y" },
                    "commit": { "hash": "111aaa222bbb" }
                },
                "destination": {
                    "branch": { "name": "develop" },
                    "commit": { "hash": "333ccc444ddd" }
                },
                "created_on": "2024-03-02T08:00:00.000000+00:00",
                "updated_on": "2024-03-02T09:15:00.000000+00:00"
            }
        ],
        "page": 1,
        "size": 2
    }"#;

    const SINGLE_PR_JSON: &str = r#"{
        "id": 42,
        "title": "Refactor auth module",
        "author": {
            "display_name": "Charlie Dev",
            "nickname": "cdev"
        },
        "source": {
            "branch": { "name": "refactor/auth" },
            "commit": { "hash": "aaa111bbb222" }
        },
        "destination": {
            "branch": { "name": "main" },
            "commit": { "hash": "ccc333ddd444" }
        },
        "created_on": "2024-04-10T14:00:00.000000+00:00",
        "updated_on": "2024-04-10T16:45:00.000000+00:00"
    }"#;

    const PR_NO_NICKNAME_JSON: &str = r#"{
        "id": 7,
        "title": "Update docs",
        "author": {
            "display_name": "External Contributor"
        },
        "source": {
            "branch": { "name": "docs/update" },
            "commit": { "hash": "eee555fff666" }
        },
        "destination": {
            "branch": { "name": "main" },
            "commit": { "hash": "ggg777hhh888" }
        },
        "created_on": "2024-05-01T09:00:00.000000+00:00",
        "updated_on": "2024-05-01T09:30:00.000000+00:00"
    }"#;

    #[test]
    fn deserialize_pr_list_response() {
        // setup
        // execute
        let paginated: BitbucketPaginatedResponse = serde_json::from_str(PR_LIST_JSON).unwrap();

        // verify
        assert_eq!(paginated.values.len(), 2);
        let pr = &paginated.values[0];
        assert_eq!(pr.id, 1);
        assert_eq!(pr.title, "Add feature X");
        assert_eq!(pr.author.nickname.as_deref(), Some("asmith"));
        assert_eq!(pr.source.branch.name, "feature/x");
        assert_eq!(pr.destination.branch.name, "main");
    }

    #[test]
    fn deserialize_single_pr() {
        // setup
        // execute
        let pr: BitbucketPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // verify
        assert_eq!(pr.id, 42);
        assert_eq!(pr.title, "Refactor auth module");
        assert_eq!(pr.source.commit.hash, "aaa111bbb222");
        assert_eq!(pr.destination.commit.hash, "ccc333ddd444");
    }

    #[test]
    fn pr_conversion_uses_nickname() {
        // setup
        let bb_pr: BitbucketPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // execute
        let pr = PullRequest::from(bb_pr);

        // verify
        assert_eq!(pr.number, 42);
        assert_eq!(pr.author, "cdev");
        assert_eq!(pr.source_branch, "refactor/auth");
        assert_eq!(pr.target_branch, "main");
        assert_eq!(pr.base_sha, "ccc333ddd444");
        assert_eq!(pr.head_sha, "aaa111bbb222");
    }

    #[test]
    fn author_fallback_to_display_name() {
        // setup
        let bb_pr: BitbucketPullRequest = serde_json::from_str(PR_NO_NICKNAME_JSON).unwrap();

        // execute
        let pr = PullRequest::from(bb_pr);

        // verify
        assert_eq!(pr.author, "External Contributor");
    }

    #[test]
    fn pr_list_conversion() {
        // setup
        let paginated: BitbucketPaginatedResponse = serde_json::from_str(PR_LIST_JSON).unwrap();

        // execute
        let prs: Vec<PullRequest> = paginated
            .values
            .into_iter()
            .map(PullRequest::from)
            .collect();

        // verify
        assert_eq!(prs.len(), 2);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "Add feature X");
        assert_eq!(prs[1].number, 2);
        assert_eq!(prs[1].title, "Fix bug Y");
        assert_eq!(prs[1].author, "bjones");
    }

    #[test]
    fn pr_timestamps_preserved() {
        // setup
        let bb_pr: BitbucketPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // execute
        let pr = PullRequest::from(bb_pr);

        // verify
        assert_eq!(pr.created_at, "2024-04-10T14:00:00.000000+00:00");
        assert_eq!(pr.updated_at, "2024-04-10T16:45:00.000000+00:00");
    }
}
