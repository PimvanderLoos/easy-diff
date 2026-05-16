# PR-0: Comment model and local storage

## Goal
Define the comment data model and store comments locally in SQLite until
submission. Users can create inline comments on specific lines/ranges, which
are persisted across sessions. Also add manual category overrides that replace
LLM classifications locally.

## Non-goals
- No comment UI in the frontend (PR-1).
- No submission to GitHub/BitBucket (PR-2).
- No right-click context menu (PR-1).

## Success Criteria
- [ ] `ReviewComment` struct: pr_id, file_path, start_line, end_line (optional for
      single-line), body, created_at, status (draft/submitted)
- [ ] `CategoryOverride` struct: pr_id, file_path, hunk_id, change_type (optional),
      attention_tags (optional), overrides LLM classification
- [ ] New SQLite tables: `review_comments` and `category_overrides`
- [ ] `CacheStore::add_comment()`, `list_comments()`, `update_comment()`,
      `delete_comment()`
- [ ] `CacheStore::set_override()`, `get_overrides()`, `delete_override()`
- [ ] Comments survive across sessions (persisted in SQLite)
- [ ] Unit tests cover CRUD for both comments and overrides
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` all pass

## Technical Approach

### Comment model (`src/cache/mod.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: i64,
    pub pr_id: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: Option<u32>,
    pub body: String,
    pub created_at: String,
    pub status: CommentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentStatus {
    Draft,
    Submitted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryOverride {
    pub id: i64,
    pub pr_id: String,
    pub file_path: String,
    pub hunk_id: String,
    pub change_type: Option<String>,
    pub attention_tags: Option<Vec<String>>,
}
```

### SQLite tables

```sql
CREATE TABLE IF NOT EXISTS review_comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pr_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft'
);

CREATE TABLE IF NOT EXISTS category_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pr_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    hunk_id TEXT NOT NULL,
    change_type TEXT,
    attention_tags TEXT,
    UNIQUE(pr_id, file_path, hunk_id)
);
```

### Tests

- `add_and_list_comments` — add 3 comments, list by PR, verify order
- `update_comment_body` — add comment, update body, verify
- `delete_comment` — add comment, delete, verify gone
- `comments_filtered_by_pr` — add to 2 PRs, list each, verify isolation
- `set_and_get_override` — set override, verify it replaces LLM classification
- `override_upserts` — set same hunk twice, verify second wins
- `delete_override` — set, delete, verify gone

## Files to Create/Modify
- `src/cache/mod.rs` — add tables, comment/override types, CRUD methods

## Dependencies
- Depends on: Epic 4 (CacheStore)
- Blocks: PR-1 (comment UI), PR-2 (submission)
