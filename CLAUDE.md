# easy-diff

PR review tool that uses LLMs to categorize, analyze, and present code changes in a more
reviewable way.

## Project Plan

See `.plan/PLAN.md` for the full project vision and feature specification.
See `.plan/ROADMAP.md` for the epic breakdown and implementation order.
See `.plan/STATE.md` for current progress — **update this file at the end of every session**.

## Tech Stack

- **Backend**: Rust (Tauri for GUI, standalone binary for CLI MVP)
- **Frontend**: Svelte + TypeScript + Vite (post-MVP, via Tauri)
- **Diff rendering** (future): CodeMirror 6
- **LLM backends**: Claude Code CLI, Codex CLI, Gemini CLI
- **Platform APIs**: GitHub REST API, BitBucket Cloud REST API
- **Storage**: SQLite (via `rusqlite`) for cache, TOML for config

## Architecture
```
src/
├── main.rs              # CLI entry point
├── config/              # Config loading (global + per-repo)
├── git/                 # Git operations (libgit2 via git2 crate)
├── platform/            # GitHub/BitBucket API clients
│   ├── github.rs
│   └── bitbucket.rs
├── llm/                 # LLM provider abstraction
│   ├── mod.rs           # Provider trait + dispatcher
│   ├── claude.rs        # Claude Code CLI integration
│   ├── codex.rs         # Codex CLI integration
│   ├── gemini.rs        # Gemini CLI integration
│   └── schema/          # JSON schemas for structured output
├── analysis/            # Two-pass analysis orchestration
│   ├── pass1_summary.rs
│   └── pass2_files.rs
├── categories/          # Category/tag system types and logic
├── cache/               # SQLite caching layer
├── diff/                # Diff parsing, filtering, formatting
└── tui/                 # Terminal UI (dialoguer/ratatui)
```

## Coding Conventions

### Rust
- Use `thiserror` for library/domain error types, `anyhow` for application-level error propagation.
- All public functions and types must have doc comments.
- Use `serde::{Serialize, Deserialize}` for all data structures that cross boundaries (config, cache, LLM I/O).
- Prefer `impl Into<String>` / `AsRef<str>` over `String` / `&str` in function signatures where it aids ergonomics.
- No `unwrap()` or `expect()` in library code. OK in tests and in `main()` for setup that truly cannot fail.
- Use `tracing` for logging, not `println!` or `eprintln!`.
- Error messages must be lowercase, no trailing punctuation (Rust convention).
- Async runtime: `tokio`. Use async for all I/O (HTTP, process spawning, file I/O where beneficial).
- Tests: use `#[cfg(test)]` modules in-file for unit tests, `tests/` directory for integration tests.
- Run `cargo clippy -- -D warnings` and `cargo fmt --check` before committing.

### Svelte (post-MVP)
- TypeScript strict mode, no `any`.
- One component per `.svelte` file.
- Use Tailwind CSS for styling.
- Invoke Tauri commands via `@tauri-apps/api/core`'s `invoke()`.

### General
- Commit messages: conventional commits (`feat:`, `fix:`, `refactor:`, `chore:`, `docs:`).
- PR plan files in `.plan/` are the spec. If implementation diverges, document the divergence in `STATE.md`, do not silently modify the plan files.
- When in doubt about a design decision, add it to the `Open Questions` section of the relevant PR plan rather than guessing.
- Add project files to git when you create them.

## STATE.md Management

At the end of every coding session, update `.plan/STATE.md` with:
1. What was completed
2. What is in progress
3. Any decisions made that diverge from the plan
4. Any new issues or tech debt introduced
5. What to work on next
