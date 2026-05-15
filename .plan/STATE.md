# easy-diff — Current State

## Status: Active Development

## Current Epic: 0 — Project Scaffold
## Current PR: PR-2

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

## Known Issues / Tech Debt
(none)

## Next Steps
- PR-2: (slot freed by CI shipping in PR-0) — available for next Epic 0 work or can be skipped.
