# PR-3: Output schema types, provider dispatcher, and config wiring

## Goal
Define the structured output types for Pass 1 and Pass 2 analysis (`Pass1Output`,
`Pass2Output`, etc.) with schemars derives so callers can generate the JSON Schema
to pass to `LlmProvider::analyze`. Implement `LlmDispatcher` which selects the
configured default provider and falls back to a secondary on error. Wire the
dispatcher factory into `main.rs`. Remove `#![allow(dead_code)]`.

## Non-goals
- No prompt construction — Epic 3.
- No actual analysis calls from `main.rs` — Epic 3.
- No caching — Epic 4.
- No category/tag enum types — those live in `src/categories/` and are an Epic 3
  concern. Schema fields use `Vec<String>` for now; Epic 3 will introduce typed
  enums and update these structs.
- No BitBucket support — Epic 6.

## Success Criteria
- [ ] `Pass1Output` and `Pass2Output` derive `JsonSchema`, `Serialize`, `Deserialize`
- [ ] `schema_for_pass1()` and `schema_for_pass2()` return `serde_json::Value`
- [ ] `LlmDispatcher::analyze()` tries default provider; on `LlmError`, logs a
      warning and tries fallback if configured; propagates error if both fail
- [ ] `create_dispatcher(config: &Config) -> Result<LlmDispatcher, LlmError>`
      constructs the right `AnyProvider` from `config.llm`
- [ ] `main.rs` calls `create_dispatcher` and logs which provider is active
- [ ] `#![allow(dead_code)]` removed from `src/llm/mod.rs`
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Output schema types (`src/llm/schema/mod.rs`)

These represent the JSON structure the LLM must return. All fields use `String` and
`Vec<String>` for now; Epic 3 replaces category fields with typed enums from
`src/categories/`.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Global (Pass 1) analysis output for an entire pull request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pass1Output {
    /// One-paragraph summary of the overall change.
    pub summary: String,
    /// Change types present across the whole PR (e.g. "feature", "refactor").
    pub change_types: Vec<String>,
    /// Attention tags raised at the PR level (e.g. "security", "breaking-change").
    pub attention_tags: Vec<String>,
    /// Logical groupings of related files.
    pub file_clusters: Vec<FileCluster>,
}

/// A named cluster of related files identified during Pass 1.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileCluster {
    /// Short label for the cluster (e.g. "Auth middleware").
    pub label: String,
    /// File paths belonging to this cluster.
    pub files: Vec<String>,
    /// One sentence explaining why these files are related.
    pub rationale: String,
}

/// Per-file (Pass 2) analysis output.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pass2Output {
    /// One-paragraph summary of changes in this file.
    pub summary: String,
    /// Change types for this file specifically.
    pub change_types: Vec<String>,
    /// Attention tags raised for this file.
    pub attention_tags: Vec<String>,
    /// Notable observations (e.g. edge cases, risks, suggestions).
    pub details: Vec<String>,
}
```

### Schema generation helpers (`src/llm/schema/mod.rs`)

```rust
/// Returns the JSON Schema for [`Pass1Output`] as a `serde_json::Value`,
/// suitable for passing directly to `LlmProvider::analyze`.
pub fn schema_for_pass1() -> serde_json::Value {
    let schema = schemars::schema_for!(Pass1Output);
    serde_json::to_value(schema).expect("Pass1Output schema is always serializable")
}

/// Returns the JSON Schema for [`Pass2Output`] as a `serde_json::Value`.
pub fn schema_for_pass2() -> serde_json::Value {
    let schema = schemars::schema_for!(Pass2Output);
    serde_json::to_value(schema).expect("Pass2Output schema is always serializable")
}
```

The `expect` calls are safe: the schema is derived by a proc-macro from a static
type — it cannot fail at runtime.

### `LlmDispatcher` (`src/llm/mod.rs`)

Wraps default + optional fallback provider. Both are `AnyProvider` (no boxing).

```rust
/// Selects and dispatches to the configured LLM provider.
///
/// Tries `default`; on any `LlmError` logs a warning and tries `fallback` if
/// configured. If fallback also fails, returns the fallback error.
pub struct LlmDispatcher {
    default: AnyProvider,
    fallback: Option<AnyProvider>,
}

impl LlmDispatcher {
    pub async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        match self.default.analyze(prompt, schema).await {
            Ok(v) => Ok(v),
            Err(e) => {
                if let Some(fb) = &self.fallback {
                    tracing::warn!(
                        error = %e,
                        fallback = fb.name(),
                        "primary provider failed, trying fallback"
                    );
                    fb.analyze(prompt, schema).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Returns the name of the active (default) provider.
    pub fn provider_name(&self) -> &str {
        self.default.name()
    }
}
```

### Dispatcher factory (`src/llm/mod.rs`)

```rust
/// Constructs an `LlmDispatcher` from resolved config.
///
/// Returns `LlmError::ProviderNotFound` if the configured provider string does
/// not match a known variant (defensive; `Provider` enum already guards this).
pub fn create_dispatcher(config: &crate::config::Config) -> LlmDispatcher {
    let default = provider_from_config(&config.llm.default_provider);
    let fallback = config.llm.fallback_provider.as_ref().map(provider_from_config);
    LlmDispatcher { default, fallback }
}

fn provider_from_config(provider: &crate::config::Provider) -> AnyProvider {
    match provider {
        crate::config::Provider::Claude => {
            AnyProvider::Claude(claude::ClaudeProvider::new(None))
        }
        crate::config::Provider::Codex => {
            AnyProvider::Codex(codex::CodexProvider::new(None))
        }
        crate::config::Provider::Gemini => {
            AnyProvider::Gemini(gemini::GeminiProvider::new(None, 3))
        }
    }
}
```

Model override from config is deferred — the `Config` struct does not yet carry
per-provider model fields. Those will be added when Epic 3 needs them.

### `main.rs` wiring

Add after config loading:

```rust
let dispatcher = llm::create_dispatcher(&config);
tracing::info!(provider = dispatcher.provider_name(), "LLM provider ready");
```

No analysis calls yet — the dispatcher is constructed and logged to confirm the
provider selection works end-to-end.

### Dead code cleanup

Remove `#![allow(dead_code)]` from `src/llm/mod.rs`. Any remaining unused items
(e.g. `validate_against_schema`, schema types not yet called from `main.rs`) get
field-level `#[allow(dead_code)]` to make the scope explicit, matching the pattern
established in Epic 1.

### Tests

#### `src/llm/schema/mod.rs`

- `schema_for_pass1_is_valid_json` — calls `schema_for_pass1()`, verifies the result
  is a JSON object with a `"type"` key (sanity check that schemars ran)
- `schema_for_pass2_is_valid_json` — same for Pass 2
- `pass1_output_round_trips` — serialize a `Pass1Output` to JSON, deserialize back,
  assert fields match
- `pass2_output_round_trips` — same for `Pass2Output`

#### `src/llm/mod.rs`

- `dispatcher_returns_default_on_success` — using a test double (closure wrapping
  `AnyProvider` logic is not easily mockable; test is a description of the contract;
  actual coverage comes from the Gemini retry tests in PR-2)
- `provider_from_config_maps_all_variants` — call `provider_from_config` for each
  `Provider` enum variant, assert `name()` matches expected string

## Files to Create/Modify
- `src/llm/schema/mod.rs` — replace stub: `Pass1Output`, `Pass2Output`, `FileCluster`,
  `schema_for_pass1()`, `schema_for_pass2()`, tests
- `src/llm/mod.rs` — add `LlmDispatcher`, `create_dispatcher()`, `provider_from_config()`;
  remove `#![allow(dead_code)]`, add field-level suppressions where needed
- `src/main.rs` — add `create_dispatcher` call and log line after config loading

## Dependencies
- Depends on: PR-0 (trait + error), PR-1 (Claude + Codex), PR-2 (Gemini + `AnyProvider`
  impl of `LlmProvider`)
- Blocks: Epic 3 (two-pass analysis engine calls `dispatcher.analyze(prompt, schema)`)

## Open Questions
- Should `create_dispatcher` accept a per-provider model override from config? Leaning
  toward deferring — the `Config` struct doesn't have per-provider model fields yet,
  and `None` (CLI default) is fine for MVP. Add `model` fields to `LlmConfig` in Epic 3
  when prompts are constructed and model selection matters.
- Should `LlmDispatcher` expose the fallback provider name for logging? Add
  `fallback_name() -> Option<&str>` only if main.rs needs it — skip for now.
- Should dispatcher retry on *any* `LlmError` or only on `ValidationFailed`? The
  current design retries on any error. `ProcessStart` errors (binary not found) will
  re-fail on fallback too, but that's acceptable — the fallback may use a different
  binary. Keep the broad retry for simplicity.
