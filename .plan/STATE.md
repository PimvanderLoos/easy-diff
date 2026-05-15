# easy-diff — Current State

## Status: Active Development

## Current Epic: 0 — Project Scaffold
## Current PR: PR-1 (config module)

## Completed
- **PR-0**: Rust binary crate initialized with full src/ module skeleton, all
  dependencies in Cargo.toml, main.rs with tracing + clap, .gitignore.
  `cargo build`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass.
  4 CLI integration tests added in `tests/cli.rs` (--help, --version, bare
  invocation, version string content). GitHub Actions CI workflow added at
  `.github/workflows/ci.yml` (build → test → clippy → fmt on every push/PR).

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

## Known Issues / Tech Debt
(none)

## Next Steps
- PR-1: Config module — define global + per-repo TOML structs and loading logic.
- PR-2: GitHub Actions CI (build, test, clippy, fmt on push/PR).
