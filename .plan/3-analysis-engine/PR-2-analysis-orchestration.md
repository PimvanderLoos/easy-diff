# PR-2: Two-pass analysis orchestration with parallel Pass 2

## Goal
Implement the analysis engine that runs Pass 1 → parallel Pass 2 across files.
Wire it into `main.rs` behind a `--analyze` flag so the full pipeline executes
end-to-end: fetch diff → run Pass 1 → run Pass 2 per file → print results.

## Non-goals
- No hunk splitting for oversized files — PR-3.
- No caching — Epic 4.
- No TUI/filtering — Epic 5.
- No custom categories — Epic 7.

## Success Criteria
- [ ] `AnalysisEngine::run(diff, config) -> Result<AnalysisResult>` orchestrates both passes
- [ ] Pass 1 runs first, produces `Pass1Output`
- [ ] Pass 2 runs in parallel per file via `tokio::JoinSet`, receives Pass 1 context
- [ ] Large-PR detection: if total diff lines > `config.preferences.large_pr_threshold`,
      Pass 1 uses file-names-only mode
- [ ] `AnalysisResult` struct holds `Pass1Output` + `HashMap<String, Pass2Output>`
- [ ] `--analyze` flag in CLI triggers the pipeline on the fetched diff
- [ ] Results printed as JSON to stdout (temporary, replaced by TUI in Epic 5)
- [ ] Concurrency limit on parallel Pass 2 (configurable, default 5)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/analysis/mod.rs`

```rust
use std::collections::HashMap;

/// Combined result of both analysis passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub pass1: Pass1Output,
    pub files: HashMap<String, Pass2Output>,
}

/// Orchestrates the two-pass analysis pipeline.
pub struct AnalysisEngine<'a> {
    dispatcher: &'a LlmDispatcher,
    large_pr_threshold: u32,
    max_concurrency: usize,
}

impl<'a> AnalysisEngine<'a> {
    pub fn new(
        dispatcher: &'a LlmDispatcher,
        large_pr_threshold: u32,
        max_concurrency: usize,
    ) -> Self { ... }

    /// Runs the full two-pass analysis.
    pub async fn run(&self, diff: &str) -> Result<AnalysisResult, anyhow::Error> { ... }
}
```

### Diff splitting

Add a simple `split_diff_by_file(diff: &str) -> Vec<(String, String)>` helper in
`src/analysis/mod.rs` that splits a unified diff into `(file_path, file_diff)` pairs.
Splits on `diff --git` headers. This is a simple text split, not a full parser
(full parser lives in Epic 5).

### Pass 1 execution

1. Count diff lines. If > threshold, set `large_pr = true`.
2. Extract file names from the diff (from `diff --git a/... b/...` headers).
3. Call `build_pass1_prompt(diff, &file_names, large_pr)`.
4. Call `dispatcher.analyze(prompt, &schema_for_pass1())`.
5. Deserialize the JSON value into `Pass1Output`.

### Pass 2 execution (parallel)

1. Split diff by file.
2. Create a `tokio::sync::Semaphore` with `max_concurrency` permits.
3. Spawn a task per file that:
   a. Acquires semaphore permit.
   b. Calls `build_pass2_prompt(path, file_diff, &pass1)`.
   c. Calls `dispatcher.analyze(prompt, &schema_for_pass2())`.
   d. Deserializes into `Pass2Output`.
4. Collect results into `HashMap<String, Pass2Output>`.
5. Log failures per file but don't abort the entire analysis.

### `main.rs` changes

Add `--analyze` flag to `Cli`. When both `--pr` and `--analyze` are given:
1. Fetch diff (existing logic).
2. Create `AnalysisEngine`.
3. Run analysis.
4. Print `AnalysisResult` as pretty JSON.

Without `--analyze`, existing behavior (print raw diff) is preserved.

### Tests

Tests for the orchestration logic are unit tests on `split_diff_by_file` and
integration tests on `AnalysisEngine` are deferred (they require real LLM calls).

#### `src/analysis/mod.rs`
- `split_diff_by_file_single_file` — one file diff → one pair
- `split_diff_by_file_multiple_files` — three file diff → three pairs
- `split_diff_by_file_empty` — empty string → empty vec
- `split_diff_by_file_preserves_content` — file diff content is intact

## Files to Create/Modify
- `src/analysis/mod.rs` — `AnalysisResult`, `AnalysisEngine`, `split_diff_by_file`, tests
- `src/main.rs` — add `--analyze` flag, wire up the analysis pipeline
- `src/llm/mod.rs` — remove `#[allow(dead_code)]` from `LlmDispatcher::analyze` and
  `LlmProvider::analyze` (now called from analysis engine)

## Dependencies
- Depends on: PR-1 (prompt construction)
- Blocks: PR-3 (hunk splitting), Epic 4 (caching), Epic 5 (CLI MVP)

## Open Questions
- Should `max_concurrency` be a config field or hardcoded? Leaning toward hardcoded
  default (5) for now, add to config in a later epic if needed.
- Should failed Pass 2 files be included in results with an error marker, or omitted?
  Leaning toward omitting and logging a warning — simpler, and the user sees the warning.
