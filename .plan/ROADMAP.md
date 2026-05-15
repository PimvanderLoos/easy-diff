# easy-diff — Roadmap

## Epic 0: Project Scaffold

Set up the Rust binary crate with the directory structure from CLAUDE.md, all core
dependencies, stub modules, CI pipeline, and the config module. After this epic,
the project compiles, lints clean, has CI running, and can load/merge configuration.

**Goals:**
- `cargo build`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass
- Directory structure matches the architecture in CLAUDE.md with stub modules
- Config structs defined and loadable from global + per-repo TOML files
- GitHub Actions CI runs build, test, clippy, and fmt on push/PR

**Non-goals:**
- No runtime functionality beyond config loading
- No tests for modules that are stubs only
- No Tauri setup (CLI-first MVP)

**Estimated PRs:** 3

---

## Epic 1: Git & Platform Integration

Detect the current git repository, parse remote URLs to identify GitHub repos, authenticate
via PAT from config, and fetch the list of open PRs. This gives the tool its primary input:
a list of PRs and their diffs.

**Goals:**
- Detect git repo from CWD using `git2`
- Parse remote URLs (HTTPS + SSH) to extract owner/repo and platform
- Fetch open PR list from GitHub REST API
- Fetch full unified diff for a selected PR
- Proper error handling for missing repo, unknown remote, auth failures

**Non-goals:**
- No BitBucket support (Epic 6)
- No TUI selection UI (Epic 5)
- No diff parsing/splitting (Epic 5)

**Estimated PRs:** 3–4

---

## Epic 2: LLM Provider Abstraction

Define the `LlmProvider` trait and implement all three backends (Claude Code CLI, Codex CLI,
Gemini CLI). Includes JSON schema generation via `schemars`, structured output parsing, and
validation with retry logic for Gemini's schema-less output.

**Goals:**
- `LlmProvider` trait with `analyze()` method
- Working implementations for Claude, Codex, and Gemini CLIs
- JSON schema definitions for Pass 1 and Pass 2 output
- Schema validation of LLM responses
- Retry + fallback logic for validation failures (especially Gemini)
- Provider selection from config (default + fallback)

**Non-goals:**
- No actual analysis prompts (those live in Epic 3)
- No token estimation or cost warnings yet
- No caching (Epic 4)

**Estimated PRs:** 4–5

---

## Epic 3: Two-Pass Analysis Engine

Build the Pass 1 (global summary) and Pass 2 (per-file) analysis pipeline. Includes
prompt construction, orchestration, parallelization of Pass 2 across files, large PR
handling, and hunk splitting for oversized files.

**Goals:**
- Pass 1 prompt construction and execution
- Pass 2 prompt construction with Pass 1 context injection
- Parallel per-file Pass 2 analysis via `tokio` tasks
- Large PR mode (file-names-only Pass 1 when diff > threshold)
- Hunk splitting for files exceeding context limits
- Category/tag types from PLAN.md §6

**Non-goals:**
- No custom categories (Epic 7)
- No caching (Epic 4)
- No UI output (Epic 5)

**Estimated PRs:** 4–5

---

## Epic 4: Caching Layer

Implement SQLite-based caching keyed by PR state, file path, provider, and schema version.
Supports full hits, partial hits (re-analyze only changed files), forced refresh, and
automatic invalidation on schema version bumps.

**Goals:**
- SQLite database setup via `rusqlite` at `.easy-diff/cache/analysis.db`
- Cache key: `(pr_id, base_sha, head_sha, file_path, provider, schema_version)`
- Store/retrieve Pass 1 and Pass 2 results
- `--refresh` flag bypasses cache
- Schema version constant with automatic invalidation

**Non-goals:**
- No "mark as viewed" (Epic 8)
- No incremental diff analysis (Epic 8)

**Estimated PRs:** 2–3

---

## Epic 5: CLI MVP

Wire everything together into the end-to-end CLI experience: TUI PR selection via
`dialoguer`, category/tag filtering, diff parsing, and filtered diff output to stdout
(pipeable to `delta`). This is the MVP milestone.

**Goals:**
- `dialoguer`-based TUI for PR selection from list
- Display Pass 1 summary in terminal
- TUI for Change Type and Attention Tag filter selection
- Diff parser that splits unified diff into per-line/per-hunk structures
- Filtered diff output to stdout with desaturated context lines
- Token estimation with confirmation prompt for large PRs
- `--refresh` flag wired through

**Non-goals:**
- No GUI
- No comment submission
- No tag color rendering beyond terminal capabilities
- No BitBucket

**Estimated PRs:** 3–4

---

## Epic 6: BitBucket Cloud Support

Add BitBucket Cloud as a second platform. Implement the REST API 2.0 client for PR listing,
diff fetching, and (later) review submission. Extend remote URL parsing to detect
`bitbucket.org`.

**Goals:**
- BitBucket REST API 2.0 client (app password auth)
- PR listing and diff fetching
- Remote URL parsing for `bitbucket.org`
- Platform auto-detection works for both GitHub and BitBucket

**Non-goals:**
- No review submission (Epic 11)
- No BitBucket-specific features beyond parity with GitHub support

**Estimated PRs:** 2–3

---

## Epic 7: Custom Categories

Allow users to define custom Change Types and Attention Tags in per-repo config. Parse the
config, inject custom category definitions into LLM prompts, and support filtering by them.

**Goals:**
- Parse `[[custom_change_types]]` and `[[custom_attention_tags]]` from per-repo config
- Inject custom definitions into Pass 1 and Pass 2 prompts
- Custom categories appear in filter UI alongside defaults
- Validation: no name collisions with built-in categories

**Non-goals:**
- No GUI color support for custom tags (Epic 10)
- No language-specific rules (Epic 12)

**Estimated PRs:** 2

---

## Epic 8: Progressive Diffs

Support incremental analysis when a PR is updated (new commits pushed). Re-analyze only
changed files, using previous analysis as context. Add "mark as viewed" tracking and
"show only unreviewed" filtering.

**Goals:**
- Detect PR updates via head SHA comparison
- Incremental diff: only re-analyze files changed between old and new head
- Provide previous analysis as context for re-analysis
- "Mark as viewed" per file/category stored in cache DB
- "Show only unreviewed" filter mode

**Non-goals:**
- No GUI indicators (Epic 10)

**Estimated PRs:** 3–4

---

## Epic 9: Tauri GUI Shell

Set up the Tauri v2 + Svelte + TypeScript + Vite frontend. Basic window, navigation
structure, and invoke plumbing to call Rust backend commands from the frontend.

**Goals:**
- Tauri v2 project setup integrated with existing Rust crate
- Svelte + TypeScript + Vite frontend scaffold
- Basic window with navigation (PR list → analysis → diff view)
- `invoke()` plumbing for core commands (list PRs, run analysis, get results)
- Tailwind CSS setup

**Non-goals:**
- No diff rendering (Epic 10)
- No review workflow (Epic 11)

**Estimated PRs:** 3

---

## Epic 10: GUI Diff Viewer

Build the CodeMirror 6-based diff viewer with syntax highlighting, category-filtered view,
desaturated context rendering, and attention tag color highlights.

**Goals:**
- CodeMirror 6 integration with diff/merge extensions
- Syntax highlighting for common languages
- Category filter sidebar with Change Type and Attention Tag toggles
- Desaturated rendering for non-matching lines
- Attention tag color highlights (configurable per tag, including custom tags)
- File cluster navigation from Pass 1 analysis

**Non-goals:**
- No inline comments (Epic 11)

**Estimated PRs:** 4–5

---

## Epic 11: Review Workflow

Add comment creation, local storage, and submission to GitHub/BitBucket. Includes the
right-click context menu for manual recategorization.

**Goals:**
- Inline comment UI (click line/range to comment)
- Local comment storage until submission
- Submit review (Approve / Request Changes / Comment) to GitHub and BitBucket APIs
- Right-click context menu: add comment, change category (manual override)
- Manual recategorization stored locally, overrides LLM classification

**Non-goals:**
- No reply threading
- No reactions/emoji

**Estimated PRs:** 4–5

---

## Epic 12: Language-Specific Rules

Add per-language LLM instruction overrides in global and per-repo config. These additional
instructions are appended to Pass 2 prompts based on the file's language.

**Goals:**
- Config parsing for `[languages.<lang>]` sections
- Language detection from file extension
- Inject `additional_instructions` into Pass 2 prompts per language
- Global + per-repo language rules with merge

**Non-goals:**
- No language-specific parsing or AST analysis

**Estimated PRs:** 2

---

## Epic 13: Advanced TUI

Build a full `ratatui`-based terminal UI as an alternative to the GUI, replicating
delta-like diff rendering with category-based highlights, tag color indicators, and
keyboard-driven navigation.

**Goals:**
- `ratatui` application with diff viewer pane
- Delta-like rendering with syntax highlighting
- Category filter panel with keyboard toggle
- Attention tag color indicators in gutter
- Keyboard-driven navigation between files, hunks, categories
- Inline summary display

**Non-goals:**
- No comment submission from TUI
- No mouse interaction

**Estimated PRs:** 4–5
