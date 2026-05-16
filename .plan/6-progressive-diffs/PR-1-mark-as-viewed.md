# PR-1: Mark-as-viewed tracking

## Goal
Track per-file "viewed" status in the cache database. Store when a file was last
viewed (at which head SHA), and surface which files have changes since last viewed
when a PR is updated.

## Non-goals
- No "show only unreviewed" filter mode (PR-2).
- No GUI indicators.
- No per-category viewed tracking (track at file level only).

## Success Criteria
- [ ] New `viewed_files` table in SQLite: `(pr_id, file_path, head_sha, viewed_at)`
- [ ] `CacheStore::mark_viewed(pr_id, file_path, head_sha)` stores a viewed record
- [ ] `CacheStore::get_viewed_sha(pr_id, file_path)` returns the head SHA at which
      the file was last viewed (or `None`)
- [ ] `CacheStore::list_viewed(pr_id)` returns all viewed records for a PR
- [ ] `AnalysisResult` gains a `has_changes_since_viewed: HashMap<String, bool>` field
      indicating which files changed since last viewed
- [ ] `AnalysisEngine` populates `has_changes_since_viewed` by comparing the viewed
      SHA per file against the current head SHA
- [ ] CLI `--mark-viewed` flag marks all displayed files as viewed after rendering
- [ ] Unit tests cover: mark/retrieve viewed, changes-since-viewed detection,
      file never viewed (always shows as changed)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/cache/mod.rs` changes

Add a new table `viewed_files` created in `create_tables()`:

```sql
CREATE TABLE IF NOT EXISTS viewed_files (
    pr_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    head_sha TEXT NOT NULL,
    viewed_at TEXT NOT NULL,
    PRIMARY KEY (pr_id, file_path)
)
```

New methods on `CacheStore`:

```rust
pub fn mark_viewed(&self, pr_id: &str, file_path: &str, head_sha: &str) -> Result<(), CacheError>
pub fn get_viewed_sha(&self, pr_id: &str, file_path: &str) -> Result<Option<String>, CacheError>
pub fn list_viewed(&self, pr_id: &str) -> Result<Vec<(String, String)>, CacheError>
```

`mark_viewed` uses `INSERT OR REPLACE` to upsert.

### `src/analysis/mod.rs` changes

Extend `AnalysisResult`:

```rust
pub struct AnalysisResult {
    pub pass1: Pass1Output,
    pub files: HashMap<String, Pass2Output>,
    /// Files that have changed since the user last viewed them.
    /// `true` = has new changes since viewed (or never viewed).
    pub has_changes_since_viewed: HashMap<String, bool>,
}
```

After analysis completes, if cache is available:
1. For each file in `result.files`, call `cache.get_viewed_sha(pr_id, file_path)`.
2. If the viewed SHA matches the current head SHA → `false` (no changes since viewed).
3. Otherwise → `true`.

### `src/main.rs` changes

Add `--mark-viewed` flag. After rendering the diff output, if the flag is set,
iterate over displayed files and call `cache.mark_viewed(...)`.

### Tests

- `mark_and_retrieve_viewed` — mark a file, retrieve, verify SHA matches
- `mark_viewed_upserts` — mark same file twice, second SHA wins
- `changes_since_viewed_detects_update` — viewed at SHA-A, current is SHA-B → true
- `no_changes_when_viewed_at_current` — viewed at current SHA → false
- `never_viewed_shows_changes` — no viewed record → true

## Files to Create/Modify
- `src/cache/mod.rs` — add `viewed_files` table and methods
- `src/analysis/mod.rs` — extend `AnalysisResult`, populate `has_changes_since_viewed`
- `src/main.rs` — add `--mark-viewed` flag

## Dependencies
- Depends on: PR-0 (incremental analysis context)
- Blocks: PR-2 (filter by unreviewed)
