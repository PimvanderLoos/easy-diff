# PR-1: TUI for PR selection and filter selection

## Goal
Replace the plain-text PR list with an interactive `dialoguer` selection menu. After
selecting a PR, display the Pass 1 summary, then present filter selection for Change Types
and Attention Tags. This is the front-end of the CLI MVP user flow.

## Non-goals
- No diff output (PR-2).
- No token estimation or cost confirmation (PR-2).
- No ratatui (Epic 13).

## Success Criteria
- [ ] `dialoguer` added as dependency
- [ ] PR selection: `Select` widget showing PR number, title, author
- [ ] After PR selection and analysis: display Pass 1 summary in terminal
- [ ] Change Type filter: `MultiSelect` showing all built-in types, all selected by default
- [ ] Attention Tag filter: `MultiSelect` showing all built-in tags, all selected by default
- [ ] Selected filters stored and available for diff output (PR-2)
- [ ] Non-interactive mode (`--pr N --analyze`) still works without TUI
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/tui/mod.rs`

```rust
/// Presents the PR list and returns the selected PR.
pub fn select_pr(prs: &[PullRequest]) -> Result<&PullRequest>

/// Displays the Pass 1 summary in the terminal.
pub fn display_summary(result: &Pass1Output)

/// Presents Change Type and Attention Tag multi-select filters.
/// Returns the selected types and tags.
pub fn select_filters() -> Result<(Vec<ChangeType>, Vec<AttentionTag>)>
```

`select_pr` uses `dialoguer::Select` with a custom formatter showing
`#{number} {title} ({author})`.

`display_summary` prints the summary paragraph, lists change types and attention tags
as bullet points, and shows file clusters with their rationales.

`select_filters` uses two `dialoguer::MultiSelect` prompts. All items default to
selected. Uses the `Display` impl on the enums for labels.

### `src/main.rs` changes

Restructure the main flow:
1. Detect repo, load config, create dispatcher.
2. If `--pr N` given, use that PR number. Otherwise, fetch PR list and use `select_pr`.
3. Fetch diff for selected PR.
4. If `--analyze` given (or default in interactive mode), run analysis.
5. Display Pass 1 summary via `display_summary`.
6. Run filter selection via `select_filters`.
7. Store filters for PR-2 to use.

For now, after filter selection, print the selected filters and exit. PR-2 adds
filtered diff output.

### Tests

TUI functions are interactive and can't be unit-tested directly. Tests focus on:
- `display_summary` output format (capture stdout in a test, or test the formatting
  function separately)
- Filter defaults (verify all variants appear in the default selection)

#### `src/tui/mod.rs`
- `all_change_types_in_defaults` — `ChangeType::all()` matches the default selection
- `all_attention_tags_in_defaults` — `AttentionTag::all()` matches the default selection

## Files to Create/Modify
- `Cargo.toml` — add `dialoguer = "0.11"`
- `src/tui/mod.rs` — replace stub: `select_pr`, `display_summary`, `select_filters`
- `src/main.rs` — restructure flow to use TUI functions

## Dependencies
- Depends on: Epic 3 (analysis engine), Epic 4 (caching)
- Blocks: PR-2 (filtered diff output)
