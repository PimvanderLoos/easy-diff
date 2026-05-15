# PR-1: GitHub REST API client

## Goal
Implement a GitHub REST API v3 client using `reqwest` that fetches the list of open
pull requests and the full unified diff for a specific PR, authenticating with the
PAT from config. Define platform-agnostic `PullRequest` and `PullRequestDiff` types
that BitBucket can reuse in Epic 6.

## Non-goals
- No BitBucket client (Epic 6).
- No PR creation, commenting, or review submission (Epic 11).
- No pagination — GitHub defaults to 30 PRs per page, sufficient for MVP.
- No diff parsing or splitting — returns the raw unified diff as a `String`.
- No retry logic beyond what `reqwest` provides — network retries can be layered later.
- No CLI changes — that's PR-2.

## Success Criteria
- [ ] Platform-agnostic `PullRequest` type with number, title, author, branches,
      timestamps
- [ ] Platform-agnostic `PullRequestDiff` type with PR number and raw diff string
- [ ] `PlatformError` enum covers auth, not found, rate limited, generic API, and
      network errors
- [ ] `GithubClient` struct wraps `reqwest::Client` and the PAT
- [ ] `list_open_pull_requests(owner, repo)` returns `Vec<PullRequest>`
- [ ] `get_pull_request_diff(owner, repo, number)` returns `PullRequestDiff`
- [ ] Auth via `Authorization: Bearer {token}` header
- [ ] `User-Agent: easy-diff` header set (required by GitHub API)
- [ ] `X-RateLimit-Remaining` logged at debug level on every response
- [ ] Rate limit exhaustion (403 + remaining = 0) returns `PlatformError::RateLimited`
- [ ] 401 responses return `PlatformError::AuthError`
- [ ] 404 responses return `PlatformError::NotFound`
- [ ] Unit tests for response deserialization and conversion
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Platform-agnostic types (`src/platform/mod.rs`)

```rust
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
```

Timestamps are `String` (ISO 8601 from the API). Adding `chrono` is deferred — the
only consumer for now is stdout display in PR-2.

### Shared error type (`src/platform/mod.rs`)

```rust
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
```

Shared across platforms — BitBucket will produce the same error variants in Epic 6.

### GitHub client (`src/platform/github.rs`)

```rust
/// GitHub REST API v3 client.
pub struct GithubClient {
    client: reqwest::Client,
    token: String,
}

impl GithubClient {
    /// Creates a new client with the given personal access token.
    pub fn new(token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("easy-diff")
            .build()
            .expect("failed to build HTTP client");
        Self { client, token: token.into() }
    }

    /// Fetches open pull requests for the given repository.
    pub async fn list_open_pull_requests(
        &self, owner: &str, repo: &str,
    ) -> Result<Vec<PullRequest>, PlatformError> { ... }

    /// Fetches the full unified diff for a specific pull request.
    pub async fn get_pull_request_diff(
        &self, owner: &str, repo: &str, pr_number: u64,
    ) -> Result<PullRequestDiff, PlatformError> { ... }
}
```

**List PRs**: `GET https://api.github.com/repos/{owner}/{repo}/pulls?state=open`
- Headers: `Authorization: Bearer {token}`, `Accept: application/vnd.github+json`
- Deserialize response body into `Vec<GithubPullRequest>` (private type)
- Convert each to platform-agnostic `PullRequest`

**Get diff**: `GET https://api.github.com/repos/{owner}/{repo}/pulls/{number}`
- Headers: `Authorization: Bearer {token}`, `Accept: application/vnd.github.diff`
- Response body is plain text (the unified diff), stored as `PullRequestDiff.diff`

**Response handling**: A shared `check_rate_limit(&response)` helper that:
1. Reads `X-RateLimit-Remaining` header, logs it at debug level via `tracing::debug!`
2. Reads `X-RateLimit-Reset` header (Unix timestamp) for the `RateLimited` error

A shared `check_status(response)` method that maps non-2xx responses:
- 401 → `PlatformError::AuthError`
- 403 + `X-RateLimit-Remaining: 0` → `PlatformError::RateLimited`
- 404 → `PlatformError::NotFound`
- Other 4xx/5xx → `PlatformError::ApiError`

### GitHub response types (private, in `src/platform/github.rs`)

```rust
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
}
```

Conversion from `GithubPullRequest` to `PullRequest` is a straightforward field
mapping implemented via `From<GithubPullRequest> for PullRequest`.

### Tests

Unit tests in `src/platform/github.rs` (`#[cfg(test)]` module):

- `deserialize_pr_list_response` — fixture JSON array → `Vec<GithubPullRequest>`,
  verify field values
- `convert_github_pr_to_pull_request` — `GithubPullRequest` → `PullRequest` via
  `From`, verify all fields map correctly
- `deserialize_single_pr` — single PR JSON object
- `deserialize_ignores_extra_fields` — JSON with fields we don't map (e.g.
  `labels`, `milestone`) deserializes without error

Fixture JSON is embedded as `const &str` in the test module, representing realistic
GitHub API responses (can be captured from
`GET https://api.github.com/repos/octocat/Hello-World/pulls`).

Error mapping is tested at the type level — verifying that specific status codes
produce the expected `PlatformError` variant. Since we can't easily mock `reqwest`
responses without a mocking crate, we test the mapping logic on a helper that takes
status + headers as input (rather than a real HTTP response).

### dead_code handling

`GithubClient` and the platform types won't be called from `main.rs` until PR-2.
Add `#![allow(dead_code)]` to `platform/mod.rs` (same pattern as `config/mod.rs`).
PR-2 removes the attribute.

## Files to Create/Modify
- `src/platform/mod.rs` — add `PullRequest`, `PullRequestDiff`, `PlatformError`
  (keep existing `pub mod` declarations and `Platform` enum from PR-0)
- `src/platform/github.rs` — replace stub with `GithubClient` implementation,
  private response types, `From` conversion, tests

## Dependencies
- Depends on: PR-0 (needs `Platform` enum in `platform/mod.rs` — shared module)
- Blocks: PR-2 (wiring calls `GithubClient` methods)

## Open Questions
- Should timestamps use `chrono::DateTime<Utc>` instead of `String`? Leaning toward
  `String` — avoids a new dependency, and the only consumer is stdout display. Can
  switch to `chrono` if a future epic needs date arithmetic.
- Should PR list pagination be supported? GitHub defaults to 30 results per page.
  Leaning toward deferring — 30 open PRs covers most repos. Pagination can be added
  if a user hits the limit.
- Should `GithubClient::new()` reject empty tokens, or let the API call fail with
  401? Leaning toward letting the API call fail — it gives a more descriptive error
  ("authentication failed") and avoids redundant validation. The caller (PR-2) checks
  for a missing token before creating the client anyway.
- Should we add `wiremock` or `mockito` as a dev dependency for HTTP mocking? Leaning
  toward no — fixture-based deserialization tests and type-level error mapping tests
  give sufficient coverage without the extra dependency. Integration tests against the
  real API can be added later (gated behind an env var).
