# easy-diff — Project Plan

## 1. Vision

Make PR reviews easier, more efficient, and higher quality by using LLMs to categorize and
analyze code changes, presenting them to the reviewer in a structured, filterable way that
separates signal from noise.

## 2. Problem Statement

Large PRs are hard to review because all changes are presented as a flat, interleaved stream:
Javadoc updates sit next to logic changes, import reordering sits next to security-sensitive
code. Reviewers waste cognitive effort on low-value changes and miss high-value ones.

easy-diff solves this by:
- Using LLMs to classify every changed line into a **Change Type** and zero or more **Attention Tags**
- Letting the reviewer filter the diff by category, seeing only what they care about at any moment
- Highlighting lines that need extra scrutiny based on LLM analysis
- Caching analysis so repeated reviews and incremental updates are fast

## 3. Non-goals

- Replacing the reviewer. The tool assists; the human decides.
- Auto-generating review comments. The LLM categorizes and flags; the human writes comments.
- Supporting non-git VCS.
- Hosting / SaaS. This is a local tool.
- IntelliJ plugin (acknowledged as a future possibility, but not designed for).

## 4. Target Platforms

- macOS (primary)
- Linux (primary)
- Windows (not a priority, but Tauri gives it for free)

## 5. Tech Stack

- **Language**: Rust
- **CLI TUI**: `dialoguer` for selection prompts, `ratatui` if we need richer TUI later
- **GUI (post-MVP)**: Tauri v2 + Svelte + TypeScript + Vite
- **Diff rendering (GUI)**: CodeMirror 6
- **Git operations**: `git2` crate (libgit2 bindings)
- **HTTP client**: `reqwest`
- **JSON**: `serde_json`, `schemars` (for generating JSON Schema from Rust types)
- **Database**: `rusqlite` (SQLite)
- **Config**: `toml` crate
- **Logging**: `tracing` + `tracing-subscriber`
- **Process spawning**: `tokio::process`
- **Diff output (CLI MVP)**: stdout in unified diff format, pipeable to `delta`

## 6. Two-Tier Category System

### 6.1 Change Types (Group 1) — Mutually exclusive per line/hunk

| Name          | Description                                                       |
|---------------|-------------------------------------------------------------------|
| Documentation | Javadocs, comments, README changes, doc-only changes              |
| Structural    | Imports, package declarations, module declarations                 |
| Chore         | Library migrations (e.g. JUnit→AssertJ), formatting, boilerplate  |
| Refactor      | Code restructuring without behavior change                        |
| Logic         | Actual behavior changes — new features, bug fixes, altered control flow |
| Uncategorized | Could not be classified into any of the above                     |

### 6.2 Attention Tags (Group 2) — Non-exclusive, zero or more per line/hunk

| Name               | Description                                                    |
|--------------------|----------------------------------------------------------------|
| Important          | Needs human verification of correctness                        |
| Error-prone        | Pattern that commonly leads to bugs                            |
| Complicated Logic  | Dense or non-obvious logic that needs careful reading           |
| Security-sensitive | Auth, crypto, input validation, SQL construction, etc.          |
| Flawed Code        | LLM believes this code has a concrete flaw                     |
| Potential Bug      | LLM believes this code may exhibit buggy behavior              |

### 6.3 Custom Categories

Users can define additional Change Types or Attention Tags in per-repo config
(`.easy-diff/config.toml`):

```toml
[[custom_change_types]]
name = "Database Migration"
instruction = "Changes to SQL migration files, ORM model definitions, or database schema"

[[custom_attention_tags]]
name = "Performance"
instruction = "Code that may have performance implications: tight loops, N+1 queries, large allocations"
color = "#FFA500"  # Used in GUI, ignored in CLI
```

### 6.4 Filtering Behavior

The UI (TUI or GUI) provides two independent filter axes:
- **Change Type filter**: show lines matching the selected Change Type(s)
- **Attention Tag filter**: show lines matching ANY of the selected Attention Tag(s)

When both filters are active, a line is shown if it matches the Change Type filter AND
the Attention Tag filter.

When only one filter axis is active, the other is treated as "any."

A line that spans multiple Change Types (edge case — should be rare since they're
mutually exclusive, but the LLM may occasionally assign two) is shown if ANY of its
assigned types match the filter.

### 6.5 Desaturated Context

When viewing a filtered diff, lines that are:
- Part of the diff but NOT matching the current filter: shown with diff colors
  (green/red) but desaturated / low-alpha, so context is preserved without distraction
- Unchanged context lines: shown normally (no diff color, no desaturation)

## 7. Two-Pass LLM Analysis

### Pass 1 — Global Summary

**Input:**
- PR title and description
- Commit messages
- List of changed files with change stats (+/- lines)
- First ~50 lines of each file's diff (for orientation)
- Repository's `.easy-diff/config.toml` custom categories (if any)
- Repository's CLAUDE.md / AGENTS.md / GEMINI.md (if present, for project context)

**Output (structured JSON):**
- `summary`: 2-5 sentence overview of what the PR does
- `strengths`: list of high-level strengths
- `weaknesses`: list of high-level concerns
- `file_clusters`: groups of related files (e.g., "backend API" → [...files],
  "frontend service" → [...files]). Used to provide cross-file context in Pass 2.
- `dependency_map`: notable cross-file dependencies (e.g., "endpoint `/api/foo`
  defined in FooController.java, consumed by foo.service.ts")

**LLM call**: single call, relatively small input.

### Pass 2 — Per-File Analysis

**Input per file:**
- Pass 1 summary + dependency_map (shared context)
- Full diff for the file
- Surrounding source code (up to a budget — configurable, default ~500 lines above/below
  changed regions, or full file if small)
- Custom category definitions

**Output per file (structured JSON):**
- For each changed line (or contiguous hunk of same-category lines):
  - `change_type`: one of the Change Type names
  - `attention_tags`: list of Attention Tag names (may be empty)
  - `note`: optional short note explaining the classification or concern
- `file_summary`: 1-2 sentence summary of what changed in this file

**LLM calls**: one per file, parallelizable across files.

### Large PR Handling

If the total diff exceeds a configurable threshold (default: 5000 lines), Pass 1 uses
only file names + commit messages (no diff snippets) to stay within context limits.

If a single file's diff exceeds context limits, split into hunks and analyze each hunk
separately, providing the Pass 1 context + file summary from previous hunks.

## 8. LLM Backend

### 8.1 Provider Abstraction

A Rust trait `LlmProvider` with:
```
fn analyze(prompt: &str, schema: &JsonSchema) -> Result<serde_json::Value>
```

Three implementations: `ClaudeCodeProvider`, `CodexProvider`, `GeminiProvider`.

The provider handles all CLI-specific invocation details and returns parsed,
validated JSON.

### 8.2 Claude Code CLI

**Invocation:**
```
claude -p --output-format json --json-schema '<schema>' '<prompt>'
```
Parses `structured_output` field from response JSON.

**Cost note:** As of June 15, 2026, `claude -p` draws from a separate credit pool
(equal to subscription price). Usage should be minimized through caching.

### 8.3 Codex CLI

**Invocation:**
```
codex exec '<prompt>' --output-schema schema.json -o result.json
```
Reads result from output file. Schema enforcement is native.

Requires `--skip-git-repo-check` since we're not running in the target repo.

### 8.4 Gemini CLI

**Invocation:**
```
gemini -p '<prompt with embedded JSON instructions>' --output-format json
```
Parses `.response` field, then parses that as JSON.

**Critical limitation:** Gemini CLI does NOT support custom output schemas (as of May
2026; this is an open feature request). The schema must be embedded in the prompt as
instructions, and the output must be validated. If validation fails, retry once with
an error correction prompt. If it fails again, fall back to a different provider or
report the error.

### 8.5 Provider Selection

Config (`~/.config/easy-diff/config.toml`):
```toml
[llm]
default_provider = "gemini"  # "claude", "codex", or "gemini"
fallback_provider = "codex"  # Used if default fails

[llm.claude]
# No additional config needed; uses `claude` from PATH

[llm.codex]
# No additional config needed; uses `codex` from PATH

[llm.gemini]
# No additional config needed; uses `gemini` from PATH
```

### 8.6 Cost Management

- **Aggressive caching** (see §9) to avoid redundant LLM calls
- **Single-shot prompts only** — no multi-turn conversations
- **User-configurable provider** so they can route to whichever subscription has capacity
- **Token estimation** before calling: estimate input size, warn if it's large,
  allow the user to confirm or skip

## 9. Caching & Progressive Diffs

### 9.1 Cache Storage

SQLite database at `.easy-diff/cache/analysis.db` (in the reviewed repo, gitignored).

### 9.2 Cache Key

```
(pr_id, base_commit_sha, head_commit_sha, file_path, provider_name, schema_version)
```

`schema_version` is a constant in the code, bumped when the analysis JSON schema changes
(invalidates all caches).

### 9.3 Scenarios

| Scenario                          | Behavior                                          |
|-----------------------------------|---------------------------------------------------|
| Exact same PR state               | Full cache hit, zero LLM calls                    |
| New commits pushed                | Fetch incremental diff (old_head..new_head). Re-analyze only changed files. Provide previous analysis + review comments as context. |
| Force refresh (`--refresh`)       | Bypass cache, re-run everything                   |
| Schema version bump               | Full cache miss, re-run everything                |

### 9.4 "Mark as Viewed"

Stored in the cache DB:
```
(file_path, change_type, head_commit_sha) → viewed_at timestamp
```

When PR updates, files with changes since last viewed show a badge/indicator.
Filtering: "show only unreviewed" mode shows only files/categories with changes
since last viewed.

### 9.5 Pass 1 Caching

Pass 1 summary is cached with key `(pr_id, base_sha, head_sha)`. If the PR
description or commit messages change but the diff doesn't, this still caches.
Acceptable trade-off — the diff is what matters for categorization.

## 10. Platform Integration

### 10.1 GitHub

- REST API v3 for PR listing, diff fetching, review submission
- Authentication: Personal Access Token (PAT) in global config
- Diff fetching: `GET /repos/{owner}/{repo}/pulls/{number}` with
  `Accept: application/vnd.github.v3.diff`

### 10.2 BitBucket Cloud

- REST API 2.0 for PR listing, diff fetching, review submission
- Authentication: App password (BitBucket's equivalent of PAT) in global config
- Diff fetching: `GET /2.0/repositories/{workspace}/{repo}/pullrequests/{id}/diff`

### 10.3 Auto-detection

When run in a git repo directory:
1. Read git remotes
2. Parse remote URL to determine platform (github.com → GitHub, bitbucket.org → BitBucket Cloud)
3. Extract owner/repo from URL
4. List open PRs for that repo

## 11. Configuration

### 11.1 Global Config

`~/.config/easy-diff/config.toml`:
```toml
[github]
token = "ghp_..."

[bitbucket]
username = "..."
app_password = "..."

[llm]
default_provider = "gemini"
fallback_provider = "codex"

[preferences]
context_lines = 5          # Lines of context around changes in diff output
max_file_context = 500     # Max lines of source context sent to LLM per file
large_pr_threshold = 5000  # Line count threshold for "large PR" mode
```

### 11.2 Per-Repo Config

`.easy-diff/config.toml` (in the reviewed repository):
```toml
# Custom Change Types and Attention Tags (see §6.3)

# Override LLM provider for this repo
[llm]
default_provider = "claude"

# Language-specific rules (post-MVP)
# [languages.java]
# additional_instructions = "Pay special attention to null safety patterns"
```

`.easy-diff/` should be added to `.gitignore` by the user (or easy-diff can offer
to do this on first run).

## 12. Review Workflow (Post-MVP, GUI)

### 12.1 Adding Comments

- Click a line or select a range → comment input appears (like GitHub)
- Comments are stored locally until submitted

### 12.2 Submitting Reviews

- When ready, submit all comments + overall status (Approve / Request Changes / Comment)
- Submits to the appropriate platform API (GitHub or BitBucket Cloud)
- Clears local comment state on successful submission

### 12.3 Right-click Context Menu

Right-click on a line in the diff viewer:
- **Add comment** → opens comment input
- **Change category** → sub-menu with:
  - Change Type: radio buttons (mutually exclusive)
  - Attention Tags: checkboxes (non-exclusive)
- Manual recategorization is stored locally and overrides LLM classification for
  that line/hunk

## 13. MVP Definition

The MVP is a **CLI tool** (no GUI) that:

1. Detects the git repository and remote platform from the current directory
2. Lists open PRs via TUI selection (arrow keys)
3. Fetches the selected PR's diff
4. Runs two-pass LLM analysis (supports Claude Code, Codex CLI, Gemini CLI)
5. Displays the PR summary in the terminal
6. Lets the user select a Change Type and/or Attention Tag filter via TUI
7. Outputs the filtered diff to stdout (pipeable to `delta`)
8. Caches analysis results (zero LLM calls on re-run with same PR state)
9. Supports `--refresh` to force re-analysis
10. Supports GitHub only (BitBucket in next phase)

### MVP non-goals
- No GUI
- No comment submission
- No "mark as viewed"
- No BitBucket support
- No attention tag color rendering (tags are computed and stored, but CLI diff
  output doesn't colorize by tag — that's a GUI feature)
- No language-specific rules
- No custom categories (uses only the defaults)

## 14. Post-MVP Roadmap (Ordered)

1. **BitBucket Cloud support** — second platform adapter
2. **Custom categories** — per-repo config for custom Change Types and Attention Tags
3. **Progressive diffs** — incremental analysis on PR updates, "mark as viewed"
4. **Tauri GUI** — diff viewer with CodeMirror 6, category filtering, tag color highlights
5. **Review workflow** — comments, review submission to GitHub/BitBucket
6. **Language-specific rules** — global + per-repo language-specific LLM instructions
7. **Incremental review UI** — "show only changes since last review" in GUI
8. **Advanced TUI** — full ratatui-based TUI as alternative to GUI (replicating
   delta-like rendering with custom highlights)

## 15. Open Questions

- Should Pass 2 analyze individual lines or contiguous hunks? Hunks are more
  natural and cheaper (fewer classifications), but lines give finer filtering.
  **Start with hunks, add line-level as a refinement if needed.**
- What's the right UX for "Uncategorized"? Ideally empty, but we need a clear
  affordance for the user to manually categorize these. In the CLI MVP, just
  show them as a separate category.
- How to handle binary files in diffs? Skip them — note in summary that binary
  files were changed but not analyzed.
- Token budget per provider? This varies by subscription tier and changes
  frequently. Don't hardcode limits; let the user configure a
  `max_input_tokens` if they want, otherwise just send and handle rate-limit
  errors gracefully.
