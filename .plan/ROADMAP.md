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

## Epic 6: Progressive Diffs

Support incremental analysis when a PR is updated (new commits pushed). Re-analyze only
changed files, using previous analysis as context. Add "mark as viewed" tracking and
"show only unreviewed" filtering.

**Goals:**
- Detect PR updates via head SHA comparison
- Incremental diff: only re-analyze files changed between old and new head
- Provide previous analysis as context for re-analysis
- "Mark as viewed" per file/category stored in cache DB
- "Show only unreviewed" filter mode
- High test coverage.

**Non-goals:**
- No GUI indicators (Epic 10)

**Estimated PRs:** 3–4

---

## Epic 7: Tauri GUI Shell + PR Selection Screen

Set up the Tauri v2 + Svelte + TypeScript + Vite frontend with the design token system
and shared component library, then implement the PR Selection screen (screen 01). After
this epic, the app launches, shows a PR list fetched from the backend, and navigates to
the review screen (stubbed) when a PR is selected.

**Design reference:** `.plan/designs/`
- `01_PRselection.png` — target for the PR selection screen
- `shared.jsx` — design tokens (`ED_TOKENS`), shared primitives (`EDAvatar`, `EDTag`,
  `SeverityDot`, `SidebarLabel`, `SidebarRow`), and color/tag helper functions
  (`edTagColor`, `edTagBg`, `CHANGE_TYPES`, `ATTENTION_TAGS`)
- `concept-b.jsx` — component structure for `BTitleBar`, `BSelection`, `BPrRow`
- `diff-data.js` — mock data structures (`PR`, `PR_LIST`) showing the shape of data
  the frontend expects from Tauri commands

The JSX files are React and use inline styles. Do NOT port them line-by-line. Use them
as structural and behavioral reference only. Translate to idiomatic Svelte (reactive
declarations, stores, `{#each}`, `{#if}`) and Tailwind CSS utility classes. Extract the
`ED_TOKENS` color values into CSS custom properties or a Tailwind theme extension so
dark/light theming works via a class toggle on `<html>`.

**Component decomposition** (each is a `.svelte` file):
- `TitleBar.svelte` — app logo, repo/PR breadcrumb, help button, user avatar
  (see `BTitleBar` in `concept-b.jsx`)
- `PrSelectionScreen.svelte` — two-column layout: filter sidebar + PR list
  (see `BSelection`)
- `PrFilterSidebar.svelte` — filter rows (Assigned to me, All open, etc.) +
  repository list (see left column of `BSelection`)
- `PrRow.svelte` — single PR row with avatar, title, branch, risky count,
  file stats, recency (see `BPrRow`)
- `Avatar.svelte` — deterministic-color monogram circle (see `EDAvatar` in `shared.jsx`)
- `Tag.svelte` — restrained chip/badge (see `EDTag`)
- `SidebarRow.svelte` — reusable sidebar list item with active state
- `SidebarLabel.svelte` — uppercase section header

**Tauri commands to expose:**
- `list_pull_requests() -> Vec<PullRequest>` — wraps existing platform client
- `get_repo_info() -> RepoInfo` — wraps existing git detection
- `get_config() -> Config` — for theme preference, provider info

**Typography:** IBM Plex Sans (UI) + IBM Plex Mono (code, metadata). Load via
`@fontsource` npm packages, not Google Fonts CDN.

**Goals:**
- Tauri v2 project setup integrated with existing Rust crate (the Svelte frontend
  lives in a `frontend/` directory; `Cargo.toml` gets Tauri feature-gated)
- Svelte + TypeScript strict mode + Vite frontend scaffold
- Tailwind CSS v4 setup with the design token palette from `shared.jsx` (`ED_TOKENS`)
  mapped to CSS custom properties, supporting light and dark themes
- IBM Plex Sans + IBM Plex Mono font integration
- Shared primitive components: `Avatar`, `Tag`, `SidebarRow`, `SidebarLabel`
- `TitleBar` component matching the design's three-column grid header
- PR selection screen matching `01_PRselection.png`: filter sidebar with counts,
  PR list with avatar, title, branch, status badges (draft/approved/changes), risky
  count with severity dot, file stats (+/-), and recency
- Tauri `invoke()` plumbing for the commands listed above
- Navigation: selecting a PR transitions to a review screen (stub/placeholder is fine)
- Dark/light theme toggle wired up (the design supports both; `shared.jsx` has
  complete token sets for each)
- An updated CI pipeline that also includes frontend tests.
- High test coverage.

**Non-goals:**
- No diff rendering (Epic 8)
- No review workflow
- No analysis triggering from the GUI (Epic 8)
- No sort controls (recent/risk/size chips are visible in the design but can be
  non-functional stubs)

**Estimated PRs:** 3–4

---

## Epic 8: GUI Diff Viewer + Inspector

Build the review screen with the custom diff viewer, category/attention filter system,
file panels with collapse/expand and review tracking, and the collapsible inspector
panel. This is the core review experience. After this epic, a user can select a PR,
see the analysis results, filter by category and attention tags, review diffs with
syntax highlighting and tag-colored stripes, mark files as reviewed, and inspect
individual hunks in the inspector panel.

**Design reference:** `.plan/designs/`
- `02_Review_nofilter.png` — review screen with no filters active, showing left rail
  (Files tab), file panels with hunk headers, and inline diff
- `03_Inspector_filteractive.png` — review screen with inspector panel open and
  filter active (dimmed non-matching lines visible)
- `concept-b.jsx` — component structure for the entire review screen:
  - `BReview` — three-column layout (left rail | main | inspector)
  - `BLeftRail` — PR meta, progress bar, Files/Filters tabs
  - `BLeftFilesTab` — show/categories/files sections with counts
  - `BLeftFiltersTab` — change type radios + attention tag checkboxes with clear button
  - `BMainToolbar` — batch selector, file navigation (prev/next), unified/split toggle
  - `BFilePanel` — collapsible file card with header, status pill, hunk sections
  - `BFileHeader` — file name, directory, +/- stats, Reviewing/Reviewed pill, kebab menu
  - `BHunkSection` — hunk header strip with `@@ L{start}-L{end}`, tag pills, confidence %
  - `BInspectorPanel` — selected change details, rationale, category, confidence bar,
    status, assignee, notes, activity log, action buttons
- `shared.jsx` — diff renderers (`EDInlineDiff`, `EDSplitDiff`), line stripe component
  (`EDLineStripe`), filter logic (`edLinePasses`, `edLineTags`), syntax highlighter
  (`edHighlight`, `edHighlightJava`), tag pill/chip components (`EDAttentionChip`,
  `EDChangeTypeChip`, `EDTagSwatch`, `BTagPill`)
- `diff-data.js` — mock data structures (`FILES`, `HUNKS`, `CLASSIFICATIONS`) showing
  the exact shape of per-file, per-hunk, and per-classification data. The Tauri commands
  should return data in a compatible shape.

**Divergence from PLAN.md:** The design uses a custom CSS Grid-based diff renderer
instead of CodeMirror 6. This is intentional — the custom renderer gives precise control
over attention-tag stripes, filter-based dimming, and hunk-level annotations that would
require complex CodeMirror 6 extensions. Document this divergence in STATE.md.

**Component decomposition:**
- `ReviewScreen.svelte` — top-level three-column grid layout, manages inspector
  open/close state
- `LeftRail.svelte` — PR metadata, progress bar, Files/Filters tab switcher
- `FilesTab.svelte` — Show section (All/Reviewed/Unreviewed counts), Categories
  section (attention tags with colored swatches + change types with outline dots
  and counts), Files section (clickable file list with +/- and tag pills)
- `FiltersTab.svelte` — Change Type radio group (with radio-dot styling from
  `BLeftFiltersTab`), Attention Tag checkbox group (with colored checkbox + swatch),
  clear button with active filter count
- `MainToolbar.svelte` — batch pill-select, file counter with prev/next arrows,
  Unified/Split segmented toggle, overflow menu, keyboard shortcut button
- `FilePanel.svelte` — rounded card container, handles collapse/expand state
- `FileHeader.svelte` — click-to-collapse header with chevron, file icon, file name +
  directory, +/- stats, Reviewing/Reviewed status pill (click toggles), kebab menu
- `FileTagRow.svelte` — collapsed-state summary: hunk count + tag pills
- `HunkSection.svelte` — hunk header strip (line range, tag pills, confidence %) +
  diff body. When a hunk is focused (clicked for inspector), highlight with accent
  left border and accent-soft background
- `DiffView.svelte` — delegates to `InlineDiff` or `SplitDiff` based on view mode
- `InlineDiff.svelte` — CSS Grid-based unified diff: 5-column grid
  (3px stripe | old line # | new line # | sign | code). See `EDInlineDiff` in
  `shared.jsx` for exact column widths and styling
- `SplitDiff.svelte` — side-by-side diff: two-column grid, each side has
  (3px stripe | line # | sign | code). Pairs adjacent del/add lines.
  See `EDSplitDiff` in `shared.jsx`
- `LineStripe.svelte` — 3px vertical bar colored by attention tags, stacked when
  multiple tags apply (see `EDLineStripe`)
- `SyntaxHighlighter.ts` — regex-based tokenizer ported from `edHighlightJava` and
  `edHighlightYaml` in `shared.jsx`. Returns token arrays with category labels;
  rendered as colored `<span>`s using syntax token colors from the design tokens
- `InspectorPanel.svelte` — right panel with sticky header ("Inspector" + close button),
  sections: Selected change (summary + line range), Why this matters (rationale),
  Category (tag pill), Confidence (% + progress bar), Status (pill select), Assignee
  (avatar + name dropdown), Notes (text input), Activity (avatar + timestamped entries),
  Actions (Override category, Mark as reviewed, Create follow-up buttons)
- `StatusPill.svelte` — Reviewing/Reviewed toggle button with checkbox icon,
  green border, filled background when reviewed (see `BStatusPill`)
- `TagPill.svelte` — uppercase dotted pill for attention tags in hunk headers
  (see `BTagPill` — "IMPORTANT", "SECURITY", etc.)
- `SegToggle.svelte` — segmented control for Unified/Split (see `SegToggle`)
- `PillSelect.svelte` — dropdown-style pill button (see `PillSelect`)
- `IconBtn.svelte` — 28×28 icon button with border (see `IconBtn`)

**Filter behavior** (critical — this is the core UX):
- Filter state is `{ changeType: string, attentionTags: string[] }`
- Change type filter: radio selection, "all" means no type filter
- Attention tag filter: checkbox multi-select, empty array means no tag filter
- When filters are active: diff lines that don't pass the filter are rendered at
  `opacity: 0.32` and `filter: saturate(0.65)` — they keep their add/del background
  colors but are visually subdued. Context lines always show at full opacity.
- A line passes if it matches the change type (or type is "all") AND has at least one
  of the selected attention tags (or no tags are selected)
- See `edLinePasses()` in `shared.jsx` for the exact logic
- Filter counts in the sidebar update live as the user toggles filters
- The Filters tab shows a "clear (N)" button when any filter is active

**Review tracking state:**
- Per-file reviewed/unreviewed status tracked in component state
- Marking a file as reviewed auto-collapses it; unmarking re-expands
  (see `toggleReviewed` in `concept-b.jsx`)
- Progress bar in left rail updates live: `{reviewed}/{total} files reviewed`
- Status pill toggles between "Reviewing" (outline) and "Reviewed" (filled green)

**Tauri commands to expose:**
- `run_analysis(pr_number: u32) -> AnalysisResult` — wraps existing analysis engine
- `get_analysis(pr_number: u32) -> Option<AnalysisResult>` — check cache
- `get_diff(pr_number: u32) -> Vec<DiffFile>` — parsed diff
- `get_pass1_summary(pr_number: u32) -> Option<Pass1Output>` — for inspector context
- Frontend types should mirror the data shapes in `diff-data.js` — particularly the
  `HUNKS` (keyed by file path → array of hunk objects with `id`, `newStart`, `lines`)
  and `CLASSIFICATIONS` (keyed by hunk id → `{ changeType, attentionTags, confidence,
  summary, rationale }`)

**Goals:**
- Review screen three-column layout matching `02_Review_nofilter.png` (without
  inspector) and `03_Inspector_filteractive.png` (with inspector open)
- Left rail with PR metadata, review progress, and tabbed Files/Filters views
- Files tab: Show section, Categories section (attention tags + change types with
  counts), Files list with per-file tag pills and +/- stats
- Filters tab: change type radio group, attention tag checkbox group, clear button
- Main toolbar with file navigation, Unified/Split toggle, batch selector
- File panels: collapsible cards with file header, status pill, hunk sections
- Custom grid-based diff renderer (inline + split modes) with:
  - Dual line number gutters (old/new)
  - `+`/`−` sign column
  - Regex-based syntax highlighting (Java + YAML; extensible)
  - 3px attention-tag color stripe on left edge of each line
  - Filter-based dimming of non-matching add/del lines
- Hunk header strips with line range, attention tag pills, confidence percentage
- Inspector panel (collapsible right column) showing selected hunk details
- File reviewed/unreviewed tracking with auto-collapse behavior
- Tauri commands wired to the existing Rust backend (analysis engine, diff parser,
  cache, platform client)
- High test coverage.

**Non-goals:**
- No comment submission or review submission to GitHub (Epic 9)
- No right-click context menu for manual recategorization (Epic 9)
- No progressive diff / incremental analysis UI (Epic 6 handles the backend)
- No "Suggest filters" button functionality (visible in design but non-functional)
- No keyboard navigation beyond browser defaults
- No sort controls in PR selection (those belong to Epic 7 and are stubbed there)

**Estimated PRs:** 5–6

---

## Epic 9: Review Workflow

Add comment creation, local storage, and submission to GitHub/BitBucket. Includes the
right-click context menu for manual recategorization.

**Goals:**
- Inline comment UI (click line/range to comment)
- Local comment storage until submission
- Submit review (Approve / Request Changes / Comment) to GitHub and BitBucket APIs
- Right-click context menu: add comment, change category (manual override)
- Manual recategorization stored locally, overrides LLM classification
- High test coverage.

**Non-goals:**
- No reply threading
- No reactions/emoji

**Estimated PRs:** 4–5

---

## Epic 10: BitBucket Cloud Support

Add BitBucket Cloud as a second platform. Implement the REST API 2.0 client for PR listing,
diff fetching, and (later) review submission. Extend remote URL parsing to detect
`bitbucket.org`.

**Goals:**
- BitBucket REST API 2.0 client (app password auth)
- PR listing and diff fetching
- Remote URL parsing for `bitbucket.org`
- Platform auto-detection works for both GitHub and BitBucket
- High test coverage.

**Non-goals:**
- No review submission (Epic 11)
- No BitBucket-specific features beyond parity with GitHub support

**Estimated PRs:** 2–3

---

## Epic 11: Language-Specific Rules

Add per-language LLM instruction overrides in global and per-repo config. These additional
instructions are appended to Pass 2 prompts based on the file's language.

**Goals:**
- Config parsing for `[languages.<lang>]` sections
- Language detection from file extension
- Inject `additional_instructions` into Pass 2 prompts per language
- Global + per-repo language rules with merge
- High test coverage.

**Non-goals:**
- No language-specific parsing or AST analysis

**Estimated PRs:** 2

---

## Epic 12: Custom Categories

Allow users to define custom Change Types and Attention Tags in per-repo config. Parse the
config, inject custom category definitions into LLM prompts, and support filtering by them.

**Goals:**
- Parse `[[custom_change_types]]` and `[[custom_attention_tags]]` from per-repo config
- Inject custom definitions into Pass 1 and Pass 2 prompts
- Custom categories appear in filter UI alongside defaults
- Validation: no name collisions with built-in categories
- High test coverage.

**Non-goals:**
- No GUI color support for custom tags (Epic 10)
- No language-specific rules (Epic 12)

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
- High test coverage.

**Non-goals:**
- No comment submission from TUI
- No mouse interaction

**Estimated PRs:** 4–5
