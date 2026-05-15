# PR-0: Initialize Rust binary crate with project structure and dependencies

## Goal
Set up the Rust binary crate with the full directory structure from CLAUDE.md, all core
dependencies, and stub modules so that `cargo build` and `cargo clippy` pass.

## Non-goals
- No runtime logic beyond `main()` printing a placeholder message.
- No Tauri setup — this is CLI-first.
- No tests beyond a trivial `#[test]` in `main.rs` to confirm the test harness works.
- No config loading logic (that's PR-1).

## Success Criteria
- [ ] `cargo build` succeeds
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] Directory structure matches the architecture in CLAUDE.md
- [ ] Each module directory has a `mod.rs` with a doc comment and re-exports (even if empty)
- [ ] `main.rs` initializes `tracing_subscriber` and logs a startup message
- [ ] All dependencies listed in CLAUDE.md tech stack are in `Cargo.toml`

## Technical Approach

### Cargo.toml
Binary crate `easy-diff`. Edition 2021. Dependencies:
- `tokio` (features: `full`) — async runtime
- `serde` (features: `derive`) + `serde_json` — serialization
- `anyhow` — application error handling
- `thiserror` — library error types
- `tracing` + `tracing-subscriber` (features: `env-filter`) — logging
- `reqwest` (features: `json`, `rustls-tls`) — HTTP client
- `git2` — libgit2 bindings
- `rusqlite` (features: `bundled`) — SQLite
- `toml` — config parsing
- `clap` (features: `derive`) — CLI argument parsing

### Directory structure
```
src/
├── main.rs
├── config/
│   └── mod.rs
├── git/
│   └── mod.rs
├── platform/
│   ├── mod.rs
│   ├── github.rs
│   └── bitbucket.rs
├── llm/
│   ├── mod.rs
│   ├── claude.rs
│   ├── codex.rs
│   ├── gemini.rs
│   └── schema/
│       └── mod.rs
├── analysis/
│   ├── mod.rs
│   ├── pass1_summary.rs
│   └── pass2_files.rs
├── categories/
│   └── mod.rs
├── cache/
│   └── mod.rs
├── diff/
│   └── mod.rs
└── tui/
    └── mod.rs
```

### Stub module pattern
Each `mod.rs` should contain:
```rust
//! Brief description of what this module does.
```

Leaf files (`github.rs`, `claude.rs`, etc.) should contain a similar doc comment only.

### main.rs
```rust
use anyhow::Result;
use clap::Parser;

/// CLI args struct (empty for now, just --help/--version)
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool")]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    let _cli = Cli::parse();
    tracing::info!("easy-diff starting");
    Ok(())
}
```

## Files to Create/Modify
- `Cargo.toml` — new, binary crate with all dependencies
- `src/main.rs` — CLI entry point with tracing init and clap
- `src/config/mod.rs` — stub
- `src/git/mod.rs` — stub
- `src/platform/mod.rs` — stub, declares `github` and `bitbucket` submodules
- `src/platform/github.rs` — stub
- `src/platform/bitbucket.rs` — stub
- `src/llm/mod.rs` — stub, declares `claude`, `codex`, `gemini`, `schema` submodules
- `src/llm/claude.rs` — stub
- `src/llm/codex.rs` — stub
- `src/llm/gemini.rs` — stub
- `src/llm/schema/mod.rs` — stub
- `src/analysis/mod.rs` — stub, declares `pass1_summary` and `pass2_files` submodules
- `src/analysis/pass1_summary.rs` — stub
- `src/analysis/pass2_files.rs` — stub
- `src/categories/mod.rs` — stub
- `src/cache/mod.rs` — stub
- `src/diff/mod.rs` — stub
- `src/tui/mod.rs` — stub
- `.gitignore` — Rust defaults (`/target`, etc.)

## Dependencies
- Depends on: nothing (first PR)
- Blocks: PR-1 (config module), PR-2 (CI)

## Open Questions
- Should we add `schemars` to Cargo.toml now (for JSON schema generation in Epic 2), or
  wait until that epic? Leaning toward adding it now to avoid a dep-only PR later.
- Should `dialoguer` and/or `ratatui` be added now as dependencies, or deferred to Epic 5?
  Leaning toward deferring — they're not needed until then, and dep lists should reflect
  actual usage.
