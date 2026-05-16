//! GitHub `gh` CLI client: PR listing and diff fetching via the `gh` command.
//!
//! Uses `gh pr list`, `gh pr view`, and `gh pr diff` with `--json` output
//! for structured data. Requires `gh` to be installed and authenticated
//! via `gh auth login`.

use serde::Deserialize;
use tokio::process::Command;

use crate::platform::{
    GithubOperations, PlatformError, PullRequest, PullRequestDiff, ReviewCommentPayload,
    ReviewEvent,
};

/// GitHub client backed by the `gh` CLI tool.
///
/// All operations shell out to `gh` using the user's existing authentication
/// session. No token management required.
#[derive(Debug)]
pub struct GhClient {
    host: Option<String>,
}

impl GhClient {
    /// Creates a new `gh`-backed client.
    ///
    /// `host` is the GitHub hostname (e.g. `"github.com"` or a GHE hostname).
    /// Pass `None` for default `github.com`.
    pub fn new(host: Option<String>) -> Self {
        Self { host }
    }

    /// Checks whether `gh` is installed and available on PATH.
    pub async fn is_installed() -> bool {
        Command::new("gh")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Checks whether `gh` is authenticated for the given host.
    pub async fn is_authenticated(host: &str) -> bool {
        Command::new("gh")
            .args(["auth", "status", "--hostname", host])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn repo_arg(&self, owner: &str, repo: &str) -> String {
        match &self.host {
            Some(host) if host != "github.com" => format!("{host}/{owner}/{repo}"),
            _ => format!("{owner}/{repo}"),
        }
    }

    async fn run_gh(&self, args: &[&str]) -> Result<String, PlatformError> {
        let output =
            Command::new("gh")
                .args(args)
                .output()
                .await
                .map_err(|e| PlatformError::ApiError {
                    status: 0,
                    message: format!("failed to execute gh: {e}"),
                })?;

        if output.status.success() {
            String::from_utf8(output.stdout).map_err(|e| PlatformError::ApiError {
                status: 0,
                message: format!("gh output is not valid UTF-8: {e}"),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(1) as u16;
            if stderr.contains("auth login") || stderr.contains("not logged") {
                Err(PlatformError::AuthError {
                    message: "gh is not authenticated — run `gh auth login` to authenticate"
                        .to_string(),
                })
            } else if stderr.contains("Could not resolve") || stderr.contains("not found") {
                Err(PlatformError::NotFound {
                    resource: stderr.trim().to_string(),
                })
            } else {
                Err(PlatformError::ApiError {
                    status: code,
                    message: stderr.trim().to_string(),
                })
            }
        }
    }
}

impl GithubOperations for GhClient {
    async fn list_open_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> {
        let repo_arg = self.repo_arg(owner, repo);
        let json_fields =
            "number,title,author,headRefName,baseRefName,headRefOid,baseRefOid,createdAt,updatedAt";
        let output = self
            .run_gh(&[
                "pr",
                "list",
                "--repo",
                &repo_arg,
                "--state",
                "open",
                "--json",
                json_fields,
            ])
            .await?;

        let gh_prs: Vec<GhPullRequest> =
            serde_json::from_str(&output).map_err(|e| PlatformError::ApiError {
                status: 0,
                message: format!("failed to parse gh pr list output: {e}"),
            })?;

        Ok(gh_prs.into_iter().map(PullRequest::from).collect())
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, PlatformError> {
        let repo_arg = self.repo_arg(owner, repo);
        let json_fields =
            "number,title,author,headRefName,baseRefName,headRefOid,baseRefOid,createdAt,updatedAt";
        let output = self
            .run_gh(&[
                "pr",
                "view",
                &pr_number.to_string(),
                "--repo",
                &repo_arg,
                "--json",
                json_fields,
            ])
            .await?;

        let gh_pr: GhPullRequest =
            serde_json::from_str(&output).map_err(|e| PlatformError::ApiError {
                status: 0,
                message: format!("failed to parse gh pr view output: {e}"),
            })?;

        Ok(PullRequest::from(gh_pr))
    }

    async fn get_pull_request_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError> {
        let repo_arg = self.repo_arg(owner, repo);
        let output = self
            .run_gh(&[
                "pr",
                "diff",
                &pr_number.to_string(),
                "--repo",
                &repo_arg,
                "--patch",
                "--color",
                "never",
            ])
            .await?;

        Ok(PullRequestDiff {
            pr_number,
            diff: output,
        })
    }

    /// Submits a pull request review via `gh pr review`.
    ///
    /// Inline comments are not supported by the `gh pr review` sub-command;
    /// when `comments` is non-empty each comment is posted separately via
    /// `gh api` using the REST API endpoint.
    async fn submit_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
        comments: Vec<ReviewCommentPayload>,
    ) -> Result<(), PlatformError> {
        let repo_arg = self.repo_arg(owner, repo);
        let pr_str = pr_number.to_string();

        // Build `gh pr review` args.
        let verdict_flag = match event {
            ReviewEvent::Approve => "--approve",
            ReviewEvent::RequestChanges => "--request-changes",
            ReviewEvent::Comment => "--comment",
        };

        let mut args: Vec<&str> = vec!["pr", "review", &pr_str, "--repo", &repo_arg, verdict_flag];

        // `gh pr review --body` accepts the body text inline.
        let body_owned: String;
        if let Some(b) = body {
            body_owned = b.to_owned();
            args.push("--body");
            args.push(&body_owned);
        }

        self.run_gh(&args).await?;

        // Post inline comments via `gh api` (gh pr review does not support them).
        for comment in &comments {
            let endpoint = format!(
                "/repos/{owner}/{repo}/pulls/{pr_number}/comments",
                owner = owner,
                repo = repo,
                pr_number = pr_number,
            );
            self.run_gh(&[
                "api",
                "--method",
                "POST",
                &endpoint,
                "--field",
                &format!("path={}", comment.path),
                "--field",
                &format!("line={}", comment.line),
                "--field",
                &format!("body={}", comment.body),
                "--field",
                "subject_type=line",
            ])
            .await
            .map_err(|e| PlatformError::ApiError {
                status: 0,
                message: format!("failed to post inline comment on {}: {e}", comment.path),
            })?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Private DTOs for `gh` JSON output
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GhPullRequest {
    number: u64,
    title: String,
    author: GhAuthor,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    login: String,
}

impl From<GhPullRequest> for PullRequest {
    fn from(pr: GhPullRequest) -> Self {
        Self {
            number: pr.number,
            title: pr.title,
            author: pr.author.login,
            source_branch: pr.head_ref_name,
            target_branch: pr.base_ref_name,
            base_sha: pr.base_ref_oid,
            head_sha: pr.head_ref_oid,
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
            "author": { "login": "octocat" },
            "headRefName": "fix/auth-bug",
            "baseRefName": "main",
            "headRefOid": "abc1234567890",
            "baseRefOid": "def5678901234",
            "createdAt": "2024-01-15T10:00:00Z",
            "updatedAt": "2024-01-15T12:00:00Z"
        },
        {
            "number": 43,
            "title": "Add feature X",
            "author": { "login": "user2" },
            "headRefName": "feat/x",
            "baseRefName": "main",
            "headRefOid": "111aaabbb",
            "baseRefOid": "222cccddd",
            "createdAt": "2024-01-16T09:00:00Z",
            "updatedAt": "2024-01-16T11:00:00Z"
        }
    ]"#;

    const SINGLE_PR_JSON: &str = r#"{
        "number": 42,
        "title": "Fix authentication bug",
        "author": { "login": "octocat" },
        "headRefName": "fix/auth-bug",
        "baseRefName": "main",
        "headRefOid": "abc1234567890",
        "baseRefOid": "def5678901234",
        "createdAt": "2024-01-15T10:00:00Z",
        "updatedAt": "2024-01-15T12:00:00Z"
    }"#;

    #[test]
    fn deserialize_gh_pr_list() {
        // setup
        // execute
        let prs: Vec<GhPullRequest> = serde_json::from_str(PR_LIST_JSON).unwrap();

        // verify
        assert_eq!(prs.len(), 2);
        assert_eq!(prs[0].number, 42);
        assert_eq!(prs[0].title, "Fix authentication bug");
        assert_eq!(prs[0].author.login, "octocat");
        assert_eq!(prs[0].head_ref_name, "fix/auth-bug");
        assert_eq!(prs[0].base_ref_name, "main");
        assert_eq!(prs[0].head_ref_oid, "abc1234567890");
        assert_eq!(prs[0].base_ref_oid, "def5678901234");
        assert_eq!(prs[1].number, 43);
    }

    #[test]
    fn deserialize_gh_single_pr() {
        // setup
        // execute
        let pr: GhPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // verify
        assert_eq!(pr.number, 42);
        assert_eq!(pr.author.login, "octocat");
        assert_eq!(pr.head_ref_oid, "abc1234567890");
        assert_eq!(pr.base_ref_oid, "def5678901234");
    }

    #[test]
    fn convert_gh_pr_to_pull_request() {
        // setup
        let gh_pr: GhPullRequest = serde_json::from_str(SINGLE_PR_JSON).unwrap();

        // execute
        let pr = PullRequest::from(gh_pr);

        // verify
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix authentication bug");
        assert_eq!(pr.author, "octocat");
        assert_eq!(pr.source_branch, "fix/auth-bug");
        assert_eq!(pr.target_branch, "main");
        assert_eq!(pr.base_sha, "def5678901234");
        assert_eq!(pr.head_sha, "abc1234567890");
        assert_eq!(pr.created_at, "2024-01-15T10:00:00Z");
        assert_eq!(pr.updated_at, "2024-01-15T12:00:00Z");
    }

    #[test]
    fn repo_arg_default_host() {
        // setup
        let client = GhClient::new(None);

        // execute
        let arg = client.repo_arg("owner", "repo");

        // verify
        assert_eq!(arg, "owner/repo");
    }

    #[test]
    fn repo_arg_github_com() {
        // setup
        let client = GhClient::new(Some("github.com".into()));

        // execute
        let arg = client.repo_arg("owner", "repo");

        // verify
        assert_eq!(arg, "owner/repo");
    }

    #[test]
    fn repo_arg_ghe_host() {
        // setup
        let client = GhClient::new(Some("github.example.com".into()));

        // execute
        let arg = client.repo_arg("owner", "repo");

        // verify
        assert_eq!(arg, "github.example.com/owner/repo");
    }
}
