# easy-diff — Current State

## Status: Active Development

## Current Epic: 1 — Git & Platform Integration
## Current PR: (none — starting Epic 1 PR-1)

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

## Known Issues / Tech Debt
(none)

## Next Steps
- Epic 1 PR-1: GitHub REST API client (`src/platform/github.rs`,
  `src/platform/mod.rs` — add `PullRequest`, `PullRequestDiff`, `PlatformError`)
