# easy-diff — Current State

## Status: Active Development

## Current Epic: 7 — Tauri GUI Shell + PR Selection Screen (COMPLETE)
## Next Epic: 8 — GUI Diff Viewer + Inspector

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

- **Epic 1 PR-2**: End-to-end wiring implemented. `main.rs` extended with
  `--pr <number>` flag; direct `git2` usage replaced by `crate::git::detect_repo_info`.
  Full flow: detect repo → load config → check platform (GitHub only) → require token
  → list PRs or fetch diff. `print_pr_list()` helper formats PRs with dynamic column
  alignment. `#![allow(dead_code)]` removed from `git/mod.rs`, `platform/mod.rs`,
  `platform/github.rs`; narrowed to field-level `#[allow(dead_code)]` on intentional
  public-API fields not yet consumed by future epics. CLI integration test
  `no_args_exits_zero` updated to `no_args_without_token_exits_nonzero` reflecting the
  new real-work behavior. All 26 tests pass. PR #6 on GitHub.

- **Epic 2 PR-0**: `LlmProvider` trait, `LlmError`, `AnyProvider` enum stub, and
  `validate_against_schema()` helper implemented in `src/llm/mod.rs`. `LlmError` has
  6 variants (ProcessStart, ProcessFailed, EmptyOutput, InvalidJson, ValidationFailed,
  Io). `LlmProvider` trait uses Rust 1.75+ async-fn-in-trait (RPITIT) — no `async-trait`
  crate needed. `AnyProvider` is an empty enum stub; variants added in PR-1/PR-2.
  `validate_against_schema()` uses `jsonschema 0.46.5` (latest; plan anticipated 0.26).
  3 unit tests cover valid value, missing required field, and wrong type. 28 tests
  total pass. PR #7 on GitHub.

- **Epic 2 PR-1**: `ClaudeProvider` and `CodexProvider` implemented in
  `src/llm/claude.rs` and `src/llm/codex.rs`. Shared `run_subprocess` (spawns
  CLI, pipes stdin/stdout, maps errors to `LlmError`) and `extract_json` (strips
  markdown fences, finds first `{`/`[`, parses JSON) added to `src/llm/mod.rs`.
  `AnyProvider` enum updated with `Claude`, `Codex`, `Gemini` variants;
  `GeminiProvider` is a struct stub (fields + `new()` only, no `LlmProvider`
  impl — added in PR-2). Clippy fix: `find(|c| c == '{' || c == '[')` →
  `find(['{', '['])`. 5 `extract_json` unit tests + 2 provider name tests added
  (32 total passing). PR #8 on GitHub.

- **Epic 2 PR-2**: Full `GeminiProvider` implementation in `src/llm/gemini.rs`.
  `retry_loop<F, Fut>` generic retry loop extracted for unit-testability (parameterized
  over fetch function; `Fn(String) -> Fut`). `build_retry_prompt` appends validation
  errors (capped at `MAX_ERRORS_IN_RETRY = 5`) to avoid context window blowup.
  `LlmProvider for AnyProvider` dispatch impl added to `src/llm/mod.rs`. Fix: `Fn`
  closure requires double-clone of `Option<String>` model — outer clone into closure,
  inner clone per invocation. 6 unit tests cover all retry scenarios + prompt
  construction. 38 total tests pass. PR #9 on GitHub.

- **Epic 2 PR-3**: Output schema types, `LlmDispatcher`, and config wiring.
  `Pass1Output`, `Pass2Output`, `FileCluster` with `#[derive(JsonSchema, Serialize,
  Deserialize)]` added to `src/llm/schema/mod.rs`. `schema_for_pass1()` /
  `schema_for_pass2()` return `serde_json::Value` via `schemars::schema_for!`.
  `LlmDispatcher { default, fallback }` and `create_dispatcher(config)` factory
  added to `src/llm/mod.rs`; `provider_from_config` maps all three `Provider` enum
  variants. `main.rs` calls `create_dispatcher` and logs the active provider at info
  level. Module-wide `#![allow(dead_code)]` removed from `src/llm/mod.rs`; replaced
  with targeted item-level suppressions across the llm module tree. 44 total tests
  pass. PR #10 on GitHub.

- **Epic 3 PR-0**: `ChangeType` (10 variants) and `AttentionTag` (7 variants) enums
  implemented in `src/categories/mod.rs`. Both derive `Serialize, Deserialize, JsonSchema,
  Clone, Copy, Debug, PartialEq, Eq, Hash` with `#[serde(rename_all = "kebab-case")]`.
  `Display` impls, `all()` methods. `Pass1Output` and `Pass2Output` updated from
  `Vec<String>` to typed enums. 56 tests pass. PR #11 on GitHub.

- **Epic 3 PR-1**: Prompt construction for Pass 1 and Pass 2. `build_pass1_prompt` with
  normal and large-PR modes. `build_pass2_prompt` with Pass 1 context injection and cluster
  membership. `description()` methods added to both category enums for prompt embedding.
  `build_category_definitions` shared helper. 10 new tests. 62 total pass. PR #12 on GitHub.

- **Epic 3 PR-2**: Two-pass analysis orchestration. `AnalysisEngine` with `run()` method
  orchestrating Pass 1 → parallel Pass 2 via `tokio::JoinSet` + `Semaphore` (concurrency=5).
  `AnalysisResult` struct. `split_diff_by_file` helper. `--analyze` CLI flag. Engine uses
  `Arc<LlmDispatcher>`. Failed files warned, not aborted. 66 total tests pass. PR #13.

- **Epic 3 PR-3**: Hunk splitting for oversized files. `split_into_chunks` splits at `@@`
  boundaries with file header preservation. `merge_pass2_outputs` deduplicates categories,
  concatenates summaries/details. `AnalysisEngine` auto-splits files exceeding
  `max_file_context` lines. 73 total tests pass. PR #14 on GitHub.

- **Epic 4 PR-0**: SQLite cache store implemented in `src/cache/mod.rs`. `CacheStore`
  wrapping `rusqlite::Connection`, `CacheKey` struct, `CacheError` enum. Two tables
  (`pass1_cache`, `pass2_cache`) with schema version filtering for auto-invalidation.
  Store/retrieve/list methods. `open_in_memory()` for tests. 7 new tests, 80 total pass.
  PR #15 on GitHub.

- **Epic 4 PR-1**: Cache integration with analysis engine. `AnalysisEngine` accepts
  `Option<CacheStore>` and `refresh: bool`. Full/partial cache hits skip LLM calls.
  `--refresh` flag bypasses reads but still writes. `PrContext` struct for cache key
  construction. `base_sha`/`head_sha` added to `PullRequest` and `GithubClient`.
  `get_pull_request()` method added. 84 total tests pass. PR #16 on GitHub.

- **Epic 5 PR-0**: Unified diff parser in `src/diff/mod.rs`. `DiffFile`, `DiffHunk`,
  `DiffLine` types. `parse_diff()` handles adds, deletes, renames, binary files,
  multi-hunk files. 10 new tests. 94 total pass. PR #17 on GitHub.

- **Epic 5 PR-1**: TUI for PR selection and filter selection via `dialoguer`. `select_pr`,
  `display_summary`, `select_filters` functions. Interactive vs non-interactive mode
  detection via `atty`. Restructured main flow. 97 total tests pass. PR #18 on GitHub.

- **Epic 5 PR-2**: Filtered diff output, token estimation — CLI MVP complete.
  `filter_files`, `render_filtered_diff` (ANSI colors), `estimate_tokens`, `confirm_large_pr`.
  Full MVP flow: select PR → estimate → confirm → analyze → summarize → filter → render.
  104 total tests pass. PR #19 on GitHub.

- **Post-MVP: `gh` CLI backend**: Added `gh`-based GitHub provider backend.
  `GithubBackend` enum (`auto`/`gh`/`api`) in config. `GhClient` in
  `src/platform/github_gh.rs` shells out to `gh pr list/view/diff --json`.
  `GithubProvider` enum + `create_github_provider` factory in
  `src/platform/github_provider.rs` selects backend based on config +
  runtime availability. `GithubOperations` trait in `src/platform/mod.rs`
  abstracts both clients. `RepoInfo` now includes `host` field for GHE
  support. 123 total tests pass (119 unit + 4 integration).

- **Post-MVP: LLM config fix + debug output**: Fixed silent config fallback
  and added failed-response debugging.
  - **Config fix**: `RawLlmConfig` now has `#[serde(deny_unknown_fields)]` —
    typos like `backend` instead of `default_provider` produce a clear parse
    error. Added per-provider sub-tables (`[llm.claude]`, `[llm.gemini]`,
    `[llm.codex]`) with `command` and `model` fields. `[llm.claude-code]`
    accepted as alias. `max_retries` and `timeout_seconds` exposed.
    `provider_from_config` now reads model from config sub-tables.
  - **Debug dump**: New `src/llm/debug.rs` with `dump_failed_response()`.
    On Pass 1/Pass 2 deserialization failure, the parsed JSON is written to
    `.easy-diff/debug/<label>_<epoch>.txt` and the file path included in the
    error message.
  - **Trace logging**: `run_subprocess()` logs raw stdout at `debug` level;
    `extract_json()` logs parsed JSON at `trace` level. Both truncate at 10KB.
  - 129 total tests pass (125 unit + 4 integration).
  - **Decisions**: `deny_unknown_fields` on `RawLlmConfig` only (not on
    top-level `RawGlobalConfig`). `default_provider` kept as canonical name.
  - **Concern**: User saw `missing field 'summary'` errors from Gemini on a
    155k-token diff and from Claude on a 7k-token diff. The debug dump
    feature will capture the actual response for diagnosis next time it occurs.

- **Epic 6 PR-0**: Incremental diff detection and selective re-analysis.
  `get_latest_head_sha` added to `CacheStore`. `changed_files_between` helper
  in `src/git/mod.rs` uses git2 tree diffs. `AnalysisEngine::run()` detects
  prior analysis at different head SHA and activates incremental mode: Pass 1
  always re-runs (with previous output as context), unchanged files reuse
  cached Pass 2 results. `build_pass1_prompt` gains `previous_summary` param.
  138 tests pass. PR #22 on GitHub.

- **Epic 6 PR-1**: Mark-as-viewed tracking. `viewed_files` table added to
  SQLite cache (`pr_id, file_path, head_sha, viewed_at`). `mark_viewed`,
  `get_viewed_sha`, `list_viewed` methods on `CacheStore`. `AnalysisResult`
  gains `has_changes_since_viewed: HashMap<String, bool>`. `--mark-viewed`
  CLI flag records current head SHA as viewed after rendering. 145 tests pass.
  PR #23 on GitHub.

- **Epic 6 PR-2**: "Show only unreviewed" filter mode. `--unreviewed` CLI flag
  filters output to files with changes since last review. `filter_unreviewed`
  in diff module. TUI `select_unreviewed_filter` prompt. Summary line:
  `[review] N of M files have changes since last review`. Clean exit when all
  reviewed. 150 tests pass. PR #24 on GitHub.

- **Epic 7 PR-0**: Tauri v2 + Svelte 5 + TypeScript + Vite 6 scaffold. Feature-gated
  `gui` Cargo feature. Tailwind CSS v4 with full ED_TOKENS palette as CSS custom
  properties. Dark/light theme toggle. IBM Plex Sans + Mono via @fontsource. CI
  updated with parallel frontend job. PR #25 on GitHub.

- **Epic 7 PR-1**: Tauri commands (`list_pull_requests`, `get_repo_info`, `get_config`)
  and shared Svelte components (Avatar, Tag, SidebarRow, SidebarLabel, SeverityDot).
  TypeScript types matching Rust structs. `Serialize` added to `PullRequest`,
  `Platform`, `RepoInfo`. PR #26 on GitHub.

- **Epic 7 PR-2**: PR selection screen matching `01_PRselection.png`. TitleBar with
  brand mark + breadcrumb. Two-column layout: PrFilterSidebar + PrSelectionScreen.
  PrRow with avatar, title, branch, status. Svelte stores for screen routing.
  Placeholder review screen on PR click. PR #27 on GitHub.

- **Epic 8 PR-0**: Tauri commands for review data + review screen layout + left rail.
  - 4 Tauri commands added to `src/gui/mod.rs`: `run_analysis`, `get_analysis`,
    `get_diff`, `get_pass1_summary`. Fixed existing `list_pull_requests` to pass
    the `host` arg to `create_github_provider` (was a pre-existing bug).
  - Frontend types added to `types.ts`: `DiffLineData`, `HunkData`, `Classification`,
    `FileEntry`, `Pass1Output`, `Pass2Output`, `AnalysisResult`, `FilterState`,
    `CHANGE_TYPES`, `ATTENTION_TAGS`, `edTagColor`, `edTagBg`.
  - New stores in `stores.ts`: `filterState`, `reviewedFiles`.
  - New Svelte components: `TagPill.svelte`, `FiltersTab.svelte`, `FilesTab.svelte`,
    `LeftRail.svelte`, `ReviewScreen.svelte`.
  - `App.svelte` wired to `ReviewScreen` replacing the placeholder.
  - 145 Rust tests pass. `cargo clippy -- -D warnings`, `cargo fmt --check` pass.
  - `npm run build` and `npm run check` both pass with 0 errors.

## In Progress
(none)

## Decisions & Divergences (Epic 8)
- **`run_analysis` uses nested runtime**: `AnalysisEngine` is `!Send` because
  `CacheStore` wraps `rusqlite::Connection` (which uses `RefCell`). Tauri
  commands require `Send` futures. Resolved by fetching diff data on the main
  async context, then moving `AnalysisEngine` construction + `engine.run()` into
  a `tokio::task::spawn_blocking` closure that builds its own
  `current_thread` runtime. No cache is passed to the GUI engine (result
  returned directly); the CLI path continues to handle caching.
- **`list_pull_requests` bug fixed**: The pre-existing `list_pull_requests` Tauri
  command was calling `create_github_provider` with only 1 argument and without
  `.await`. Fixed as part of Epic 8 PR-0 since it was discovered during the
  compilation pass.
- **FilesTab drops filter props**: `FilesTab` no longer accepts `filter` /
  `onFilterChange` props (they were unused). Filter interaction belongs to
  `FiltersTab`. The parent `LeftRail` passes both to `FiltersTab` only.

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

- **Epic 1 PR-2 — field-level `#[allow(dead_code)]` instead of module-wide**: Removing
  `#![allow(dead_code)]` produced warnings for public API fields not yet read from
  `main.rs` (`remote_name`, `remote_url`, `source_branch`, `target_branch`,
  `created_at`, `pr_number`). Suppressed at field level rather than restoring
  module-wide suppression — makes the scope of "intentionally unused" explicit.
- **Epic 1 PR-2 — `no_args_exits_zero` test renamed**: The old test assumed a no-op
  invocation. Now that bare invocation does real work (repo detection → token check),
  the test was renamed to `no_args_without_token_exits_nonzero` and asserts non-zero
  exit with a message mentioning "token" or "git repository."

- **Epic 2 PR-0 — RPITIT instead of `async-trait`**: `LlmProvider::analyze()` uses
  Rust 1.75+ return-position impl Trait in trait (RPITIT). This avoids the `async-trait`
  crate but makes the trait not object-safe. Object-safe dispatch is handled via the
  `AnyProvider` enum (PR-1/PR-2 add variants, PR-3 implements `LlmProvider for AnyProvider`).
- **Epic 2 PR-0 — `jsonschema 0.46.5` vs plan's `0.26`**: The plan specified `0.26`
  but `cargo add jsonschema` resolved to `0.46.5`. The API (`validator_for` / `iter_errors`)
  is identical — the version bump pulled a larger dependency tree but is otherwise a no-op.
- **Epic 2 PR-0 — `validate_against_schema` stays in `mod.rs`**: Open question resolved
  as "keep in `mod.rs`" — single function, no benefit to a separate submodule.
- **Epic 2 PR-0 — `analyze()` accepts `&str` prompt**: Open question resolved as
  "plain `&str`" — structure (system vs user prompt separation) deferred to Epic 3.

- **Epic 2 PR-3 — module-wide `#![allow(dead_code)]` on submodule files**: The spec
  said to use item-level suppressions. For `claude.rs`, `codex.rs`, and `gemini.rs`,
  all items are uniformly unreachable from `main()` until Epic 3, so module-wide
  suppression was used on those files for maintainability. Items in `mod.rs` and
  `schema/mod.rs` got per-item `#[allow(dead_code)]` as specified.
- **Epic 2 PR-3 — model overrides deferred**: `provider_from_config` hardcodes
  `model: None` for all providers. `LlmConfig` does not yet have per-provider model
  fields. Will be added in Epic 3 when prompts are constructed and model selection
  matters.
- **Epic 2 PR-3 — `fallback_name()` skipped**: `LlmDispatcher` does not expose a
  `fallback_name() -> Option<&str>` method; `main.rs` doesn't need it. Can be added
  later if logging the fallback at startup becomes useful.

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

## Known Issues / Tech Debt (Epic 2)
- **Epic 2 PR-0 — module docstring example is `no_run`**: The example in `src/llm/mod.rs`
  is not compiled by `cargo test`. Acceptable for now — the example references types
  (concrete providers) that don't exist yet.
- **Epic 2 PR-1 — CLI flags unverified**: `claude --print --output-format json` and
  `codex --quiet` flags are taken from the plan spec but not tested against real
  installed binaries. Subprocess integration tests (requiring actual binaries) are
  out of scope for CI; verify flags manually before relying on the output.
- **Epic 2 PR-1 — `extract_json` byte-offset seek**: `find(['{', '['])` locates the
  first brace by byte position, not JSON structural analysis. Pathological inputs
  (e.g. prose containing `{` before the actual JSON object) could misparse. Acceptable
  for MVP; harden with fixture tests once real CLI output is observed.
- **Epic 2 PR-2 — Gemini CLI flags unverified**: `gemini [--model ...]` invocation
  not tested against a real `gemini` binary. Same risk as Claude/Codex flags above.
- **Epic 2 PR-2 — no backoff between retries**: Retry loop calls `fetch` immediately
  without sleep. Acceptable for local CLI subprocesses (no rate limits), but worth
  noting if Gemini CLI ever gains network-rate-limit behavior.

## Decisions & Divergences (Epic 7)
- **Custom CSS diff renderer**: Per ROADMAP, the design uses a custom CSS Grid-based
  diff renderer instead of CodeMirror 6 (documented in Epic 8 section).
- **Feature gate over workspace**: Tauri integration uses `gui` feature flag rather
  than a separate workspace member for simplicity.
- **Missing PR metadata**: `files`, `adds`, `dels`, `risky` not available from
  `list_pull_requests` API — displayed as "—" in the UI. Future enrichment possible.

## Next Steps
- Epic 8 PR-1: Custom grid-based diff renderer (InlineDiff + SplitDiff), MainToolbar,
  FilePanel, FileHeader, HunkSection, get_diff wired into ReviewScreen.

## Post-MVP Addons Completed
- **`gh` CLI backend** — GitHub operations can now prefer `gh auth login`
  credentials over a PAT. Config: `[github] backend = "auto" | "gh" | "api"`.
  Files: `src/platform/github_gh.rs`, `src/platform/github_provider.rs`,
  `src/platform/mod.rs`, `src/config/mod.rs`, `src/git/mod.rs`, `src/main.rs`,
  `tests/cli.rs`.
