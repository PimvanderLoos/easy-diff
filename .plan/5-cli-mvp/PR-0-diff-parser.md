# PR-0: Unified diff parser

## Goal
Implement a proper unified diff parser in `src/diff/mod.rs` that parses raw unified diff
text into structured types: `DiffFile`, `DiffHunk`, `DiffLine`. This replaces the
simple `split_diff_by_file` text splitter from Epic 3 with a full parser needed for
filtered output and the TUI.

## Non-goals
- No diff rendering or display — PR-2.
- No filtering logic — PR-2.
- No syntax highlighting (Epic 10).

## Success Criteria
- [ ] `DiffFile` struct: path, old_path (for renames), hunks
- [ ] `DiffHunk` struct: header, old_start, old_count, new_start, new_count, lines
- [ ] `DiffLine` enum: Context, Added, Removed (with line content)
- [ ] `parse_diff(diff: &str) -> Vec<DiffFile>` parses full unified diff
- [ ] Handles: added files, deleted files, renamed files, binary files (skip)
- [ ] Handles multiple hunks per file
- [ ] Preserves line numbers (old and new)
- [ ] All types derive `Debug, Clone, Serialize, Deserialize`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/diff/mod.rs`

```rust
/// A parsed file within a unified diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub old_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
    pub is_new: bool,
    pub is_deleted: bool,
}

/// A single hunk within a file diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

/// A single line within a hunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

/// Parses a full unified diff into structured file entries.
pub fn parse_diff(diff: &str) -> Vec<DiffFile>
```

Parsing strategy:
1. Split on `diff --git a/... b/...` headers.
2. Parse file metadata: `--- a/...`, `+++ b/...`, detect `/dev/null` for adds/deletes.
3. Parse hunk headers: `@@ -old_start,old_count +new_start,new_count @@`.
4. Classify lines: ` ` = context, `+` = added, `-` = removed.
5. Skip binary file markers (`Binary files ... differ`).
6. Handle rename detection from `rename from`/`rename to` or `similarity index` headers.

### Tests

Use realistic diff fixtures as string constants.

- `parse_single_file_single_hunk` — minimal diff → 1 file, 1 hunk
- `parse_single_file_multiple_hunks` — 2 hunks in one file
- `parse_multiple_files` — 3 files → 3 `DiffFile` entries
- `parse_new_file` — `/dev/null` as old → `is_new = true`
- `parse_deleted_file` — `/dev/null` as new → `is_deleted = true`
- `parse_rename` — rename detection → `old_path` set
- `parse_binary_skipped` — binary file marker → file present but no hunks
- `parse_empty_diff` — empty string → empty vec
- `line_classification` — context/added/removed lines correctly typed
- `hunk_numbers_parsed` — old_start/count, new_start/count match header

## Files to Create/Modify
- `src/diff/mod.rs` — replace stub: all types, `parse_diff`, tests

## Dependencies
- Depends on: nothing directly (standalone parser)
- Blocks: PR-2 (filtered diff output)
