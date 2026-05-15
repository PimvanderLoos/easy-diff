# easy-diff — Current State

## Status: Active Development

## Current Epic: 1 — Git & Platform Integration
## Current PR: (none — starting Epic 1 PR-2)

## Completed
- **PR-0**: Rust binary crate initialized with full src/ module skeleton, all
  dependencies in Cargo.toml, main.rs with tracing + clap, .gitignore.
  `cargo build`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass.
  4 CLI integration tests added in `tests/cli.rs` (--help, --version, bare
  invocation, version string content). GitHub Actions CI workflow added at
  `.github/workflows/ci.yml` (build → test → clippy → fmt on every push/PR).
- **PR-1**: Config module implemented in `src/config/mod.rs`. Public API:
  `Config`, `GithubConfig`, `BitbucketConfig`, `LlmConfig`, `Preferences`,
  `Provider` enum. Two-struct pattern: all-Option raw deserialization structs
  merged into concrete public types. Three-layer merge: hardcoded defaults <
  global (`~/.config/easy-diff/config.toml`) < per-repo
  (`.easy-diff/config.toml`). `ConfigError` via `thiserror`. 7 unit tests.
  `main.rs` wires up config loading and logs resolved provider at info level.
  Added deps: `dirs = "5"`, dev-dep `tempfile = "3"`. PR #2 on GitHub.
- **PR-2**: CI workflow aligned to spec. Replaced `actions/cache@v4` with
  `Swatinem/rust-cache@v2`. Switched build step to `cargo build --all-targets`.
  Removed push branch filter (now triggers on every branch). Job renamed to
  `check` / "Build & Check" per spec. `RUST_BACKTRACE` env var removed (not in
  spec). `cargo audit` deferred per spec guidance. PR #3 on GitHub.
- **Epic 1 PR-0**: Git repository detection implemented in `src/git/mod.rs`.
  `Platform` enum (GitHub/BitBucket) with `Display` impl added to
  `src/platform/mod.rs`. `RepoInfo` struct, `GitError` enum (6 variants),
  `detect_repo_info()`, and `parse_remote_url()` implemented. Handles HTTPS,
  SSH SCP-style, and SSH protocol URL formats. 11 unit tests: all URL
  format/error cases covered plus a tempdir integration test. `#![allow(dead_code)]`
  added (removed in Epic 1 PR-2 when wired into main.rs). PR #4 on GitHub.
- **Epic 1 PR-1**: GitHub REST API client implemented. `PullRequest`,
  `PullRequestDiff`, and `PlatformError` platform-agnostic types added to
  `src/platform/mod.rs`. `GithubClient` implemented in `src/platform/github.rs`
  with `list_open_pull_requests` and `get_pull_request_diff`. Private GitHub
  response types (`GithubPullRequest`, `GithubUser`, `GithubRef`) and
  `From<GithubPullRequest> for PullRequest` conversion implemented. Shared
  `check_rate_limit` (logs `X-RateLimit-Remaining` at debug level) and
  `check_status` helpers map 401/403+ratelimit/404/other to `PlatformError`
  variants. 4 unit tests: list deserialization, field conversion, single PR
  deserialization, extra-field tolerance. `#![allow(dead_code)]` on both
  `platform/mod.rs` and `platform/github.rs` (removed in PR-2).

## In Progress
(none)

## Decisions & Divergences
- **schemars added in PR-0**: The open question in PR-0 asked whether to add
  `schemars` now or defer. Decision: added now (0.8.x) to avoid a dep-only PR
  later when Epic 2 needs it.
- **dialoguer / ratatui deferred**: Not added to Cargo.toml yet; will be added
  in Epic 5 when the TUI layer is built.
- **Rust updated**: Cargo 1.82.0 could not handle edition2024 in `clap_lex`
  1.1.0 (a transitive dep of clap 4.6+). Resolved by running `rustup update
  stable` → Rust 1.95.0. No code change required.
- **CI shipped in PR-0, not PR-2**: The roadmap planned CI as a separate PR-2.
  It was added alongside PR-0 because the workflow is trivial and gives
  immediate feedback on the open PR. PR-2 slot is now available for other
  Epic 0 work (or can be skipped).
- **Integration tests added beyond PR-0 spec**: PR-0 spec called for one
  trivial `#[test]` in main.rs. Replaced with four proper CLI integration
  tests in `tests/cli.rs` that exercise the built binary.
- **`#![allow(dead_code)]` in config module**: In a binary crate, public
  fields not yet read from `main` trigger dead_code errors under `-D warnings`.
  Suppressed module-wide; fields are intentional public API for future epics.
- **`ConfigError` uses `String` for path, not `PathBuf`**: `PathBuf` doesn't
  impl `Display`, so thiserror can't interpolate it in `#[error(...)]`. Path
  is converted via `path.display().to_string()` at error construction time.
- **Per-repo token overrides not supported**: Open question in PR-1 spec.
  Tokens (GitHub, Bitbucket) are global-only; `RawRepoConfig` exposes only
  `llm` and `preferences` overrides.
- **PR-2 CI was partially shipped in PR-0**: PR-0 included a functional CI
  workflow but used `actions/cache@v4` with manual paths, a push branch filter,
  and `--locked` flags. PR-2 replaced this with the spec-prescribed setup:
  `Swatinem/rust-cache@v2`, push on any branch, `cargo build --all-targets`.
- **`cargo audit` deferred**: Open question in PR-2 spec resolved as "later" —
  the spec itself leaned toward deferral; adds tool install overhead for minimal
  benefit at the current project size.
- **`Swatinem/rust-cache` not pinned**: Using `@v2` tracking latest v2.x as
  the spec recommended.
- **Epic 1 PR-0 — `origin` hardcoded**: Open question resolved as "hardcode
  origin" — overwhelmingly common case; a `remote_name` config field can be
  added later if needed.
- **Epic 1 PR-0 — `Platform` errors on unknown hosts**: Open question resolved
  as "error, not `Unknown(String)` variant" — an unknown platform is
  unsupported; clear error beats silent no-op, new platforms are added to enum.
- **Epic 1 PR-0 — `detect_repo_info` takes a path parameter**: Open question
  resolved as "take path for testability" — callers pass `"."`.
- **Epic 1 PR-1 — timestamps stay `String`**: Open question resolved as
  "defer `chrono`" — ISO 8601 strings from the API are passed through as-is;
  the only consumer (PR-2 stdout display) doesn't need date arithmetic.
- **Epic 1 PR-1 — no pagination**: Open question resolved as "defer" — GitHub
  defaults to 30 results per page, sufficient for MVP. Pagination can be added
  if a user hits the limit.
- **Epic 1 PR-1 — empty token accepted at construction**: Open question
  resolved as "let the API call fail" — `GithubClient::new()` accepts any
  string; an empty/invalid token produces `PlatformError::AuthError` on the
  first API call, which gives a descriptive error. The caller (PR-2) validates
  token presence before constructing the client.
- **Epic 1 PR-1 — no HTTP mocking library**: Open question resolved as "no" —
  fixture-based deserialization tests and type-level `From` conversion tests
  give sufficient unit coverage without `wiremock` or `mockito`.
- **Epic 1 PR-1 — `#![allow(dead_code)]` also added to `github.rs`**: The
  spec mentioned adding it to `platform/mod.rs` only, but `github.rs` also has
  dead code (GithubClient methods unreachable from main). Added to both files;
  both are removed in PR-2.

## Known Issues / Tech Debt
- **Epic 1 PR-0 — SCP detector is a heuristic**: `parse_remote_url` detects
  SSH SCP-style URLs via `url.contains('@') && url.contains(':')`. A URL like
  `user:pass@host/path` would be misrouted into the SCP branch. In practice
  git remotes never take this form, so the risk is negligible.
- **Epic 1 PR-0 — no test for SSH protocol without user prefix**: The code
  handles `ssh://github.com/owner/repo` (no `user@`) via the `find('@')`
  fallback, but there is no explicit test for this case.
- **Epic 1 PR-1 — `NotFound.resource` is a raw URL path**: The `resource`
  field in `PlatformError::NotFound` is populated with the request URL path
  (e.g. `/repos/owner/repo/pulls/42`) rather than a human-readable description.
  Functional but not ideal UX; should be improved when error messages are
  displayed to the user in PR-2.
- **Epic 1 PR-1 — 403 without `X-RateLimit-Remaining: 0` maps to `ApiError`**:
  A 403 caused by SAML enforcement, repo permissions, or other non-rate-limit
  reasons falls through to `PlatformError::ApiError`. This is correct behaviour
  but worth noting — the raw API response body will be included in the error
  message.

## Next Steps
- Epic 1 PR-2: End-to-end wiring — `main.rs` detects repo, loads config,
  fetches open PRs from GitHub, prints them. `--pr <number>` flag fetches and
  displays the raw unified diff. Removes direct `git2` usage from `main.rs`.
  Removes `#![allow(dead_code)]` from `git/mod.rs`, `platform/mod.rs`, and
  `platform/github.rs`.
