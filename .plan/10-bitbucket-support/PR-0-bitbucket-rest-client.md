# PR-0: BitBucket Cloud REST API client

## Goal
Implement a BitBucket Cloud REST API 2.0 client that can list open pull
requests and fetch PR diffs, matching the capabilities already provided by
`GithubClient` / `GhClient` for GitHub.

## Non-goals
- No review submission (roadmap says Epic 11).
- No `bb` CLI wrapper (no widely-used equivalent to `gh`).
- No BitBucket Server/self-hosted (Cloud only).
- No GUI wiring (PR-1).

## Success Criteria
- [ ] `BitbucketClient` struct in `src/platform/bitbucket.rs` with `new(username, app_password)`
- [ ] `list_open_pull_requests(workspace, repo_slug)` → `Vec<PullRequest>`
- [ ] `get_pull_request(workspace, repo_slug, pr_id)` → `PullRequest`
- [ ] `get_pull_request_diff(workspace, repo_slug, pr_id)` → `PullRequestDiff`
- [ ] `BitbucketOperations` trait (mirrors `GithubOperations` minus `submit_review`)
- [ ] Auth via HTTP Basic (username + app password) per BitBucket Cloud docs
- [ ] Map BitBucket JSON to the shared `PullRequest` / `PullRequestDiff` types
- [ ] Unit tests for JSON deserialization (fixture-based, same approach as GitHub)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes

## Technical Approach

### BitBucket REST API 2.0 endpoints

- List PRs: `GET /2.0/repositories/{workspace}/{repo_slug}/pullrequests?state=OPEN`
- Get PR: `GET /2.0/repositories/{workspace}/{repo_slug}/pullrequests/{id}`
- Get diff: `GET /2.0/repositories/{workspace}/{repo_slug}/pullrequests/{id}/diff`

Base URL: `https://api.bitbucket.org`

### `BitbucketClient` (`src/platform/bitbucket.rs`)

```rust
pub struct BitbucketClient {
    http: reqwest::Client,
    username: String,
    app_password: String,
}
```

Authentication: HTTP Basic auth header on every request (`username:app_password`).

### Response mapping

BitBucket PR JSON shape (simplified):
```json
{
  "id": 1,
  "title": "...",
  "author": { "display_name": "...", "nickname": "..." },
  "source": { "branch": { "name": "..." }, "commit": { "hash": "..." } },
  "destination": { "branch": { "name": "..." }, "commit": { "hash": "..." } },
  "created_on": "2024-01-01T00:00:00.000000+00:00",
  "updated_on": "2024-01-01T00:00:00.000000+00:00"
}
```

Maps to `PullRequest`:
- `number` ← `id`
- `title` ← `title`
- `author` ← `author.nickname` (fallback to `display_name`)
- `source_branch` ← `source.branch.name`
- `target_branch` ← `destination.branch.name`
- `base_sha` ← `destination.commit.hash`
- `head_sha` ← `source.commit.hash`
- `created_at` ← `created_on`
- `updated_at` ← `updated_on`

### `BitbucketOperations` trait

```rust
#[allow(async_fn_in_trait)]
pub trait BitbucketOperations {
    async fn list_open_pull_requests(
        &self, workspace: &str, repo_slug: &str,
    ) -> Result<Vec<PullRequest>, PlatformError>;

    async fn get_pull_request(
        &self, workspace: &str, repo_slug: &str, pr_id: u64,
    ) -> Result<PullRequest, PlatformError>;

    async fn get_pull_request_diff(
        &self, workspace: &str, repo_slug: &str, pr_id: u64,
    ) -> Result<PullRequestDiff, PlatformError>;
}
```

### Error mapping

- 401 → `PlatformError::AuthError`
- 404 → `PlatformError::NotFound`
- 429 → `PlatformError::RateLimited`
- Other 4xx/5xx → `PlatformError::ApiError`

### Tests

- `list_prs_deserialization` — parse fixture JSON, verify field mapping
- `single_pr_deserialization` — parse single PR fixture
- `author_fallback_to_display_name` — nickname missing, uses display_name
- `error_status_mapping` — verify 401/404/429 handling

## Files to Create/Modify
- `src/platform/bitbucket.rs` — full implementation (replace stub)
- `src/platform/mod.rs` — add `BitbucketOperations` trait

## Dependencies
- Depends on: Epic 1 (platform types, PlatformError)
- Blocks: PR-1 (platform routing)
