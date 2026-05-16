# PR-2: "Show only unreviewed" filter mode

## Goal
Add a filter mode that shows only files with changes since last viewed. Integrate
with the existing TUI filter selection and the CLI diff rendering pipeline.

## Non-goals
- No GUI indicators (future epic).
- No per-hunk viewed tracking (file-level only).
- No automatic mark-as-viewed on display (that's the existing `--mark-viewed` flag).

## Success Criteria
- [ ] `--unreviewed` CLI flag filters the file list to only unreviewed files
- [ ] TUI filter selection includes an "Unreviewed only" option when viewed data exists
- [ ] Filtered diff output only includes files where `has_changes_since_viewed == true`
- [ ] When `--unreviewed` is active and no files are unreviewed, display a message
      ("All files have been reviewed at the current revision")
- [ ] Combining `--unreviewed` with category filters works (intersection)
- [ ] Summary output shows unreviewed count: "3 of 12 files have changes since last review"
- [ ] Unit tests cover: filter with some reviewed, filter with all reviewed, filter
      combined with category filters
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/diff/mod.rs` changes

Add a filtering function:

```rust
/// Filters a list of DiffFiles to only those that have changes since viewed.
pub fn filter_unreviewed(
    files: &[DiffFile],
    changes_since_viewed: &HashMap<String, bool>,
) -> Vec<&DiffFile>
```

### `src/tui/mod.rs` changes

Extend `select_filters` to include an "Unreviewed only" toggle when the analysis
result contains viewed data. This adds a boolean to the filter state.

### `src/main.rs` changes

1. Add `--unreviewed` CLI flag.
2. After analysis, if `--unreviewed` is set or TUI selects it:
   - Filter `result.files` to only those with `has_changes_since_viewed == true`.
   - Filter the parsed diff to only those files.
   - If empty, print the "all reviewed" message and exit cleanly.
3. Category filters apply after the unreviewed filter (intersection).

### Display enhancement

When viewed data exists, print a summary line before the diff:

```
📋 3 of 12 files have changes since last review
```

(Using plain text, not emoji — per CLAUDE.md conventions.)

Actually: `[review] 3 of 12 files have changes since last review`

### Tests

- `filter_unreviewed_returns_only_changed` — 3 files, 1 reviewed at current → returns 2
- `filter_unreviewed_empty_when_all_current` — all viewed at current → empty
- `filter_combined_with_category` — unreviewed + category filter = intersection
- `unreviewed_flag_in_cli` — integration test verifying flag is accepted

## Files to Create/Modify
- `src/diff/mod.rs` — add `filter_unreviewed`
- `src/tui/mod.rs` — extend filter selection
- `src/main.rs` — add `--unreviewed` flag, wire filter logic

## Dependencies
- Depends on: PR-1 (mark as viewed, `has_changes_since_viewed` field)
- Blocks: nothing (epic complete after this PR)
