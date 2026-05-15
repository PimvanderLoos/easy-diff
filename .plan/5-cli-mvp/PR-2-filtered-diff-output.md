# PR-2: Filtered diff output and token estimation

## Goal
Complete the CLI MVP: apply the selected Change Type and Attention Tag filters to the
analysis results, output only matching file diffs to stdout (pipeable to `delta`), add
token estimation with confirmation for large PRs, and wire `--refresh` through the full
flow.

## Non-goals
- No syntax highlighting (Epic 10).
- No ratatui TUI (Epic 13).
- No comment submission (Epic 11).
- No custom categories (Epic 7).

## Success Criteria
- [ ] Filter analysis results by selected Change Types and Attention Tags
- [ ] Output filtered unified diff to stdout (valid unified diff format)
- [ ] Context lines from non-matching hunks are desaturated (dimmed ANSI)
- [ ] File headers include the per-file summary and tags as comments
- [ ] Token estimation: count diff lines, estimate tokens (~4 chars/token), show to user
- [ ] Confirmation prompt for large PRs (> threshold): "This PR has ~X tokens. Continue?"
- [ ] `--refresh` flag wired through from CLI to analysis engine to cache
- [ ] Exit cleanly if user declines the large-PR confirmation
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/diff/mod.rs` additions

```rust
/// Filters parsed diff files to include only those matching the given
/// change types or attention tags from the analysis results.
pub fn filter_files(
    files: &[DiffFile],
    analysis: &HashMap<String, Pass2Output>,
    change_types: &[ChangeType],
    attention_tags: &[AttentionTag],
) -> Vec<DiffFile>
```

A file is included if its analysis result contains at least one matching change type
OR at least one matching attention tag.

### `src/diff/mod.rs` — formatted output

```rust
/// Renders filtered diff files as unified diff text with ANSI color.
///
/// Added lines: green. Removed lines: red. Context lines: dim.
/// File headers: bold, with per-file summary as a comment line.
pub fn render_filtered_diff(
    files: &[DiffFile],
    analysis: &HashMap<String, Pass2Output>,
) -> String
```

### Token estimation

```rust
/// Estimates the token count for a diff string.
/// Uses a simple heuristic: ~4 characters per token.
pub fn estimate_tokens(diff: &str) -> usize {
    diff.len() / 4
}
```

### `src/tui/mod.rs` additions

```rust
/// Shows token estimate and asks for confirmation on large PRs.
/// Returns true if the user confirms (or PR is under threshold).
pub fn confirm_large_pr(estimated_tokens: usize, threshold_lines: u32) -> Result<bool>
```

### `src/main.rs` — full flow

Complete the MVP flow:
1. Detect repo → config → dispatcher → cache.
2. Select PR (TUI or `--pr`).
3. Fetch diff.
4. Estimate tokens. If large, confirm with user.
5. Run analysis (with cache, respecting `--refresh`).
6. Display Pass 1 summary.
7. Select filters.
8. Filter files by analysis results + selected filters.
9. Render filtered diff to stdout.

### Tests

#### `src/diff/mod.rs`
- `filter_files_includes_matching` — file with matching change type → included
- `filter_files_excludes_non_matching` — file with no matching types/tags → excluded
- `filter_files_matches_on_tag` — file with matching attention tag → included
- `estimate_tokens_basic` — 400 chars → ~100 tokens

#### `src/tui/mod.rs`
- Formatting tests for summary display (non-interactive part)

## Files to Create/Modify
- `src/diff/mod.rs` — add `filter_files`, `render_filtered_diff`, `estimate_tokens`, tests
- `src/tui/mod.rs` — add `confirm_large_pr`
- `src/main.rs` — complete the MVP flow
- `Cargo.toml` — may need `colored` or `termcolor` for ANSI output (or use raw ANSI escapes)

## Dependencies
- Depends on: PR-0 (diff parser), PR-1 (TUI)
- Blocks: nothing — this is the CLI MVP milestone

## Open Questions
- Should `delta` integration be explicit (pipe detection) or just output valid unified diff
  and let the user pipe? Leaning toward just outputting valid diff — `delta` auto-detects.
- Should context lines be dimmed or omitted? Leaning toward dimmed — preserves context
  while highlighting what matters.
