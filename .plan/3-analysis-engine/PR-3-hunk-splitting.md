# PR-3: Hunk splitting for oversized files

## Goal
When a single file's diff exceeds `config.preferences.max_file_context` lines, split
it into smaller chunks (one per hunk or group of hunks) and analyze each chunk
separately. Merge the per-chunk Pass 2 results into a single `Pass2Output` for the file.

## Non-goals
- No full diff parser (Epic 5 builds the complete parser).
- No caching — Epic 4.
- No custom categories — Epic 7.

## Success Criteria
- [ ] `split_into_chunks(file_diff: &str, max_lines: u32) -> Vec<String>` splits oversized
      diffs at hunk boundaries (`@@ ... @@`)
- [ ] Chunks include the file header (`--- a/...` / `+++ b/...`) for context
- [ ] `AnalysisEngine` detects oversized files and splits before Pass 2
- [ ] Multiple chunk results are merged: union of change_types/attention_tags, concatenated
      summaries and details
- [ ] Files under the threshold are unchanged (no splitting)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/analysis/mod.rs` additions

```rust
/// Splits a file diff into chunks of at most `max_lines` lines each,
/// breaking at hunk boundaries (`@@ ... @@`).
///
/// Each chunk retains the file header (`--- a/...` / `+++ b/...`) so the
/// LLM has file context. If the diff has fewer than `max_lines` lines,
/// returns a single-element vec with the original diff.
pub fn split_into_chunks(file_diff: &str, max_lines: u32) -> Vec<String>
```

Logic:
1. Extract the file header (lines before first `@@`).
2. Split remaining lines at each `@@` boundary.
3. Greedily group hunks until adding the next would exceed `max_lines`.
4. Prepend the file header to each chunk.

### Merge logic

```rust
/// Merges multiple `Pass2Output` results from chunked analysis into one.
///
/// Change types and attention tags are de-duplicated (union). Summaries
/// and details are concatenated.
pub fn merge_pass2_outputs(outputs: Vec<Pass2Output>) -> Pass2Output
```

### `AnalysisEngine` integration

In the Pass 2 loop, before calling the LLM:
1. Check if `file_diff.lines().count() > max_file_context`.
2. If yes, call `split_into_chunks` and analyze each chunk.
3. Call `merge_pass2_outputs` on the results.
4. If no, analyze the file diff as-is (existing path).

### Tests

#### `src/analysis/mod.rs`
- `split_into_chunks_short_diff_returns_one` — under threshold → single chunk
- `split_into_chunks_splits_at_hunk_boundary` — multi-hunk diff splits correctly
- `split_into_chunks_preserves_file_header` — each chunk starts with header
- `split_into_chunks_respects_max_lines` — no chunk exceeds limit (except single
  huge hunks which can't be split further)
- `merge_pass2_deduplicates_categories` — duplicate change types removed
- `merge_pass2_concatenates_summaries` — summaries joined with space
- `merge_pass2_concatenates_details` — details from all chunks present

## Files to Create/Modify
- `src/analysis/mod.rs` — add `split_into_chunks`, `merge_pass2_outputs`, integrate
  into `AnalysisEngine`, tests

## Dependencies
- Depends on: PR-2 (analysis orchestration)
- Blocks: Epic 5 (CLI MVP uses the complete analysis engine)
