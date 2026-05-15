# PR-1: Cache integration with analysis engine and `--refresh` flag

## Goal
Wire the cache into the analysis pipeline. Before running analysis, check the cache.
On cache hit, skip the LLM call. On partial hit (some files cached, some not), only
analyze uncached files. Add `--refresh` CLI flag to bypass the cache entirely.

## Non-goals
- No incremental diff detection (Epic 8).
- No cache eviction or size limits.
- No "mark as viewed" (Epic 8).

## Success Criteria
- [ ] `AnalysisEngine` accepts an optional `CacheStore` reference
- [ ] Full cache hit: Pass 1 + all Pass 2 files cached → no LLM calls
- [ ] Partial cache hit: only uncached files sent to LLM
- [ ] Analysis results stored in cache after successful LLM calls
- [ ] `--refresh` flag skips cache reads (but still writes results)
- [ ] Cache key constructed from PR metadata (number, base SHA, head SHA, provider name)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/analysis/mod.rs` changes

Add `cache: Option<&CacheStore>` and `refresh: bool` to `AnalysisEngine`.

Modified `run()` flow:
1. Build `CacheKey` from PR metadata.
2. If `!refresh`, try `cache.get_pass1(key)`.
3. On hit, use cached Pass 1. On miss, run LLM and store result.
4. For Pass 2, get list of cached files. Only spawn tasks for uncached files.
5. After each successful Pass 2 LLM call, store result.
6. Merge cached + fresh results.

### `src/main.rs` changes

1. Add `--refresh` flag to `Cli`.
2. When `--analyze` is active, open `CacheStore` at repo cache path.
3. Pass cache and refresh flag to `AnalysisEngine`.
4. `AnalysisEngine` needs PR metadata (number, base_sha, head_sha) for the cache key.
   Add these as parameters to `run()` or as a separate `PrContext` struct.

### `PrContext` struct

```rust
/// PR metadata needed for cache keying and analysis context.
pub struct PrContext {
    pub pr_number: u64,
    pub base_sha: String,
    pub head_sha: String,
}
```

The GitHub API already returns these fields. Wire them from `PullRequest` into `PrContext`.

### Tests

Unit tests use an in-memory cache and mock/stub the dispatcher.

- `full_cache_hit_skips_llm` — pre-populate cache, verify no LLM calls
  (This is a contract description; actual test uses a counting wrapper or simply
  verifies the result matches cached data without needing the LLM binary.)
- `refresh_flag_bypasses_cache` — pre-populate cache, set refresh=true, verify
  result is re-analyzed (different from cached value)
- `partial_hit_only_analyzes_uncached_files` — cache 2 of 3 files, verify only
  1 LLM call for Pass 2

Since these tests require mocking the LLM dispatcher (which uses subprocess calls),
they are structured as integration-style tests that verify cache read/write behavior
independently of LLM calls.

## Files to Create/Modify
- `src/analysis/mod.rs` — add cache integration, `PrContext`, modify `AnalysisEngine`
- `src/main.rs` — add `--refresh` flag, open cache, pass to analysis engine
- `src/platform/mod.rs` — ensure `PullRequest` exposes base/head SHA fields
  (may already be present from Epic 1)

## Dependencies
- Depends on: PR-0 (cache store)
- Blocks: Epic 5 (CLI MVP uses cached analysis)

## Open Questions
- Should cache writes happen eagerly (after each file) or in batch (after all files)?
  Leaning toward eagerly — if the process crashes mid-analysis, partial results are saved.
