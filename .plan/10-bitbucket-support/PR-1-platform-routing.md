# PR-1: Platform routing and CLI/GUI integration

## Goal
Wire the BitBucket client into the application so that repositories hosted on
bitbucket.org are handled automatically. Both CLI (`main.rs`) and GUI
(`src/gui/mod.rs`) should detect the platform from the remote URL and use the
appropriate client.

## Non-goals
- No review submission for BitBucket (Epic 11).
- No BitBucket-specific TUI prompts beyond what GitHub already has.
- No `submit_review` Tauri command for BitBucket.

## Success Criteria
- [ ] `main.rs` no longer rejects BitBucket repos — routes to `BitbucketClient`
- [ ] `create_platform_client()` factory dispatches by `Platform` enum
- [ ] GUI `list_pull_requests` Tauri command works for BitBucket repos
- [ ] GUI `get_diff` and `run_analysis` work for BitBucket repos
- [ ] `submit_review` Tauri command returns a clear "not supported" error for BitBucket
- [ ] Config: `[bitbucket] username = "..." app_password = "..."` required for BitBucket
- [ ] Missing BitBucket credentials produce a clear error message
- [ ] Integration test: `bitbucket_platform_rejected_without_credentials`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes

## Technical Approach

### Platform client abstraction

Create a `PlatformClient` enum in `src/platform/mod.rs`:

```rust
pub enum PlatformClient {
    Github(GithubProvider),
    Bitbucket(BitbucketClient),
}
```

With methods that delegate based on variant:
- `list_open_pull_requests(owner_or_workspace, repo)`
- `get_pull_request(owner_or_workspace, repo, number)`
- `get_pull_request_diff(owner_or_workspace, repo, number)`

### Factory function

```rust
pub fn create_platform_client(
    platform: Platform,
    config: &Config,
    host: &str,
) -> Result<PlatformClient, PlatformError>
```

- `Platform::GitHub` → creates `GithubProvider` (existing logic from `create_github_provider`)
- `Platform::BitBucket` → creates `BitbucketClient` from config credentials

### `main.rs` changes

Remove the `if repo_info.platform != Platform::GitHub` rejection. Replace the
direct `create_github_provider` call with `create_platform_client`. The rest
of the flow (list PRs, get diff, analyze) uses the `PlatformClient` methods.

### GUI changes (`src/gui/mod.rs`)

- `list_pull_requests`: use `create_platform_client` instead of `create_github_provider`
- `get_diff`: same
- `submit_review`: check platform, return error for BitBucket

### Tests

- `create_platform_client_github` — verify dispatch
- `create_platform_client_bitbucket_missing_creds` — verify clear error
- `bitbucket_submit_review_unsupported` — verify error message

## Files to Create/Modify
- `src/platform/mod.rs` — add `PlatformClient` enum and factory
- `src/main.rs` — replace GitHub-specific logic with platform-generic
- `src/gui/mod.rs` — update Tauri commands to use `PlatformClient`
- `tests/cli.rs` — add credential-missing test (if feasible without network)

## Dependencies
- Depends on: PR-0 (BitBucketClient)
- Blocks: nothing (epic complete after this PR)
