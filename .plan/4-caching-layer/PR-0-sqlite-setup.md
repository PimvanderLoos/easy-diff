# PR-0: SQLite database setup and cache store

## Goal
Implement the SQLite caching layer that stores and retrieves Pass 1 and Pass 2 analysis
results. The cache is keyed by PR identity, file path, provider, and schema version.
Supports full hits (skip analysis entirely) and partial hits (re-analyze only changed files).

## Non-goals
- No `--refresh` flag wiring — PR-1.
- No incremental diff detection (Epic 8).
- No "mark as viewed" (Epic 8).
- No schema migration tooling beyond version-based invalidation.

## Success Criteria
- [ ] `CacheStore` struct wraps `rusqlite::Connection`
- [ ] `CacheStore::open(path)` creates or opens the DB, runs `CREATE TABLE IF NOT EXISTS`
- [ ] Schema: `(pr_id TEXT, base_sha TEXT, head_sha TEXT, file_path TEXT, provider TEXT,
      schema_version INTEGER, result_json TEXT, created_at TEXT)`
- [ ] `store_pass1(key, result) -> Result<()>`
- [ ] `get_pass1(key) -> Result<Option<Pass1Output>>`
- [ ] `store_pass2(key, file_path, result) -> Result<()>`
- [ ] `get_pass2(key, file_path) -> Result<Option<Pass2Output>>`
- [ ] `get_cached_files(key) -> Result<Vec<String>>` — returns file paths that have
      cached Pass 2 results for this key
- [ ] Schema version constant; cache miss when version doesn't match
- [ ] `CacheError` enum via `thiserror`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/cache/mod.rs`

```rust
use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 1;

/// Cache key identifying a unique analysis context.
#[derive(Debug, Clone)]
pub struct CacheKey {
    pub pr_id: String,
    pub base_sha: String,
    pub head_sha: String,
    pub provider: String,
}

/// SQLite-backed analysis cache.
pub struct CacheStore {
    conn: Connection,
}
```

Two tables:
- `pass1_cache`: `(pr_id, base_sha, head_sha, provider, schema_version, result_json, created_at)`
  Primary key: `(pr_id, base_sha, head_sha, provider, schema_version)`
- `pass2_cache`: same key + `file_path`
  Primary key: `(pr_id, base_sha, head_sha, file_path, provider, schema_version)`

Queries filter on `schema_version = SCHEMA_VERSION` so bumping the constant
automatically invalidates old entries.

### Database location

Default path: `.easy-diff/cache/analysis.db` relative to the repo root.
`CacheStore::open` creates parent directories if needed.

### Tests

All tests use an in-memory SQLite database (`Connection::open_in_memory()`).

- `store_and_retrieve_pass1` — round-trip
- `store_and_retrieve_pass2` — round-trip
- `get_pass1_returns_none_on_miss` — different key → None
- `get_pass2_returns_none_on_miss` — different file → None
- `schema_version_mismatch_returns_none` — store with version 1, query pretending version 2 → None
- `get_cached_files_returns_stored_paths` — store 3 files, get list
- `overwrite_updates_existing` — store twice with same key, get latest

## Files to Create/Modify
- `src/cache/mod.rs` — replace stub: `CacheStore`, `CacheKey`, `CacheError`, all methods, tests

## Dependencies
- Depends on: Epic 3 (schema types `Pass1Output`, `Pass2Output`)
- Blocks: PR-1 (`--refresh` wiring), Epic 5 (CLI uses cache)
