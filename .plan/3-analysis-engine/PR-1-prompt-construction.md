# PR-1: Prompt construction for Pass 1 and Pass 2

## Goal
Build the prompt templates for both analysis passes. Pass 1 receives the full diff
(or file-name list for large PRs) and produces a global summary with categories.
Pass 2 receives a single file's diff plus the Pass 1 context and produces per-file
analysis. Both prompts embed the category definitions from `src/categories/`.

## Non-goals
- No analysis orchestration or LLM calls — PR-2.
- No parallel execution — PR-2.
- No hunk splitting for oversized files — PR-3.
- No custom categories (Epic 7).

## Success Criteria
- [ ] `build_pass1_prompt(diff: &str, file_names: &[&str], large_pr: bool) -> String`
- [ ] `build_pass2_prompt(file_path: &str, file_diff: &str, pass1_summary: &Pass1Output) -> String`
- [ ] Both prompts embed the full list of `ChangeType` and `AttentionTag` variants with descriptions
- [ ] Pass 1 large-PR mode uses file-names-only instead of full diff
- [ ] Pass 2 prompt includes the Pass 1 summary for context
- [ ] Unit tests verify prompt content (contains expected sections, category names)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/analysis/pass1_summary.rs`

```rust
/// Builds the Pass 1 prompt for global PR analysis.
///
/// In normal mode, includes the full unified diff. In large-PR mode
/// (`large_pr = true`), includes only the file names to avoid exceeding
/// context limits.
pub fn build_pass1_prompt(diff: &str, file_names: &[&str], large_pr: bool) -> String
```

The prompt structure:
1. System instruction: "You are a code review assistant analyzing a pull request."
2. Category definitions: list all `ChangeType` and `AttentionTag` variants with
   one-line descriptions.
3. Output format instruction: "Respond with a JSON object matching this schema: ..."
   Include a brief description of each field.
4. The diff content (or file list in large-PR mode).

Category descriptions are defined as associated constants or a helper function
in `src/categories/mod.rs` — e.g. `ChangeType::description(&self) -> &'static str`.

### `src/analysis/pass2_files.rs`

```rust
/// Builds the Pass 2 prompt for a single file.
///
/// Includes the Pass 1 summary as context so the per-file analysis is
/// coherent with the global view.
pub fn build_pass2_prompt(
    file_path: &str,
    file_diff: &str,
    pass1_summary: &Pass1Output,
) -> String
```

The prompt structure:
1. System instruction: "You are analyzing a single file from a pull request."
2. Pass 1 context: summary, change types, and the cluster this file belongs to (if any).
3. Category definitions (same as Pass 1).
4. Output format instruction.
5. File path and diff content.

### `src/categories/mod.rs` additions

Add `description() -> &'static str` method to both `ChangeType` and `AttentionTag`.
These are short (< 15 word) descriptions embedded in prompts.

### Tests

#### `src/analysis/pass1_summary.rs`
- `pass1_prompt_contains_diff` — normal mode includes the diff text
- `pass1_prompt_large_pr_uses_file_names` — large-PR mode includes file names, not diff
- `pass1_prompt_contains_all_change_types` — all `ChangeType` variant names appear
- `pass1_prompt_contains_all_attention_tags` — all `AttentionTag` variant names appear

#### `src/analysis/pass2_files.rs`
- `pass2_prompt_contains_file_path` — includes the file path
- `pass2_prompt_contains_file_diff` — includes the diff text
- `pass2_prompt_contains_pass1_summary` — includes the Pass 1 summary text
- `pass2_prompt_contains_category_definitions` — change types and tags present

## Files to Create/Modify
- `src/categories/mod.rs` — add `description()` methods to both enums
- `src/analysis/pass1_summary.rs` — replace stub: `build_pass1_prompt`, tests
- `src/analysis/pass2_files.rs` — replace stub: `build_pass2_prompt`, tests

## Dependencies
- Depends on: PR-0 (category types, updated schema types)
- Blocks: PR-2 (orchestration calls these functions)
