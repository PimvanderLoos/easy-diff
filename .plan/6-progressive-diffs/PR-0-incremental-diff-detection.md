# PR-0: Incremental diff detection and selective re-analysis

## Goal
Detect when a PR has been updated (new commits pushed) by comparing the cached
head SHA against the current head SHA. When an update is detected, compute which
files changed between the old and new head, and re-analyze only those files while
preserving cached results for unchanged files.

## Non-goals
- No "mark as viewed" tracking (PR-1).
- No "show only unreviewed" filtering (PR-2).
- No GUI indicators.

## Success Criteria
- [ ] `AnalysisEngine` detects a previous analysis for the same PR (same `pr_id`,
      same `base_sha`, different `head_sha`) and triggers incremental mode
- [ ] Only files that changed between old_head and new_head are re-analyzed
- [ ] Unchanged files reuse their cached Pass 2 results from the previous head
- [ ] Pass 1 is always re-run in incremental mode (the summary may change)
- [ ] Previous Pass 1 output is provided as context in the re-analysis prompt
- [ ] Cache store supports querying the most recent head SHA for a given PR
- [ ] New unit tests cover: full incremental hit, partial file changes, no
      previous analysis (falls back to full analysis)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/cache/mod.rs` changes

Add a new method to `CacheStore`:

```rust
/// Returns the most recent head_sha for which a Pass 1 entry exists for
/// this PR+provider combination. Returns `None` if no previous analysis.
pub fn get_latest_head_sha(&self, pr_id: &str, base_sha: &str, provider: &str) -> Result<Option<String>, CacheError>
```

This queries `pass1_cache` for rows matching `(pr_id, base_sha, provider)` ordered
by rowid desc, returning the head_sha. This lets the engine discover a previous
analysis to build incrementally from.

### `src/analysis/mod.rs` changes

Add an `incremental` path in `run()`:

1. Before running the normal flow, call `cache.get_latest_head_sha(...)`.
2. If a previous head SHA exists and differs from `ctx.head_sha`:
   - Fetch the previous Pass 1 output from cache using the old key.
   - Compute the set of changed files between old_head and new_head using
     `git diff --name-only old_head new_head` (via a new helper).
   - For files NOT in the changed set, load their cached Pass 2 from the old key.
   - For files IN the changed set (or new files), run Pass 2 as normal.
   - Always re-run Pass 1 (with previous Pass 1 as context hint in prompt).
3. If no previous head SHA exists, fall through to the existing full-analysis path.

### `src/git/mod.rs` changes

Add a helper function:

```rust
/// Returns the list of file paths that changed between two commits.
pub fn changed_files_between(repo_path: &Path, old_sha: &str, new_sha: &str) -> Result<Vec<String>, GitError>
```

Uses `git2` to compute a diff between the two tree objects and extract paths.

### Prompt changes (`src/analysis/pass1_summary.rs`)

Add an optional `previous_summary` parameter to `build_pass1_prompt`. When provided,
append a section:

```
## Previous Analysis Context
The following is the previous analysis of this PR (before the latest commits):
<previous summary JSON>

Update the analysis to reflect the new changes while preserving accuracy for
unchanged portions.
```

### Tests

- `incremental_reuses_unchanged_files` — cache results for 3 files at old_head,
  mark 1 file as changed, verify only 1 file gets re-analyzed
- `incremental_reruns_pass1` — verify Pass 1 is always called even when some
  files are cached
- `no_previous_analysis_does_full_run` — no cache entries, verify full analysis
- `changed_files_between` unit tests with a temp git repo

## Files to Create/Modify
- `src/cache/mod.rs` — add `get_latest_head_sha`
- `src/analysis/mod.rs` — add incremental detection logic in `run()`
- `src/analysis/pass1_summary.rs` — add `previous_summary` param
- `src/git/mod.rs` — add `changed_files_between`

## Dependencies
- Depends on: Epic 4 (cache), Epic 3 (analysis engine)
- Blocks: PR-1 (mark as viewed needs incremental awareness)

## Open Questions
- Should incremental mode also re-run Pass 2 for files whose classification
  *references* a changed file (e.g. via dependency_map)? Leaning toward no for
  now — keep it simple, only re-analyze files with actual diff changes.
