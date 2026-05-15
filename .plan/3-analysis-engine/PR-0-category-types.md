# PR-0: Category and tag type definitions

## Goal
Define the built-in Change Type and Attention Tag enums in `src/categories/mod.rs`.
These typed enums replace the `Vec<String>` placeholders in `Pass1Output` and `Pass2Output`,
giving compile-time guarantees on category values the LLM can return. Update the schema
types to use the new enums.

## Non-goals
- No custom categories (Epic 7).
- No prompt construction — PR-1.
- No analysis orchestration — PR-2/PR-3.
- No filtering or display logic — Epic 5.

## Success Criteria
- [ ] `ChangeType` enum with variants: Feature, BugFix, Refactor, Test, Docs, Style,
      Chore, Performance, Security, Dependency
- [ ] `AttentionTag` enum with variants: Security, BreakingChange, NeedsTest, Complexity,
      OffTopic, Nitpick, DesignDecision
- [ ] Both enums derive `Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, Hash`
- [ ] Both enums have `Display` impls returning kebab-case (e.g. `"bug-fix"`, `"breaking-change"`)
- [ ] `Pass1Output` and `Pass2Output` updated: `change_types: Vec<ChangeType>`,
      `attention_tags: Vec<AttentionTag>`
- [ ] Existing schema tests updated and still pass
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### `src/categories/mod.rs`

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Classification of the type of change a file (or PR) represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeType {
    Feature,
    BugFix,
    Refactor,
    Test,
    Docs,
    Style,
    Chore,
    Performance,
    Security,
    Dependency,
}

/// Tags that flag areas requiring reviewer attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AttentionTag {
    Security,
    BreakingChange,
    NeedsTest,
    Complexity,
    OffTopic,
    Nitpick,
    DesignDecision,
}
```

Add `Display` impls that match the serde kebab-case names.

Add `ChangeType::all()` and `AttentionTag::all()` returning `&'static [Self]` for
prompt construction and filtering.

### `src/llm/schema/mod.rs` updates

Replace `Vec<String>` fields with the typed enums:
- `Pass1Output.change_types: Vec<ChangeType>`
- `Pass1Output.attention_tags: Vec<AttentionTag>`
- `Pass2Output.change_types: Vec<ChangeType>`
- `Pass2Output.attention_tags: Vec<AttentionTag>`

Import from `crate::categories`.

### Tests

#### `src/categories/mod.rs`
- `change_type_serializes_to_kebab_case` — `serde_json::to_string(&ChangeType::BugFix)` → `"bug-fix"`
- `attention_tag_serializes_to_kebab_case` — `serde_json::to_string(&AttentionTag::BreakingChange)` → `"breaking-change"`
- `change_type_display_matches_serde` — `format!("{}", ChangeType::BugFix)` == `"bug-fix"`
- `change_type_round_trips` — serialize + deserialize all variants
- `attention_tag_round_trips` — serialize + deserialize all variants
- `all_change_types_returns_all_variants` — `ChangeType::all().len()` == variant count
- `all_attention_tags_returns_all_variants` — same for tags

#### Update `src/llm/schema/mod.rs` tests
- `pass1_output_round_trips` — update to use `ChangeType::Feature` / `AttentionTag::Security`
- `pass1_output_rejects_missing_required_field` — still valid, no change needed

## Files to Create/Modify
- `src/categories/mod.rs` — replace stub: `ChangeType`, `AttentionTag`, `Display` impls, `all()`, tests
- `src/llm/schema/mod.rs` — update `Pass1Output`, `Pass2Output` to use typed enums; update tests

## Dependencies
- Depends on: Epic 2 PR-3 (schema types exist)
- Blocks: PR-1 (prompt construction needs category definitions)
