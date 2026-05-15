# PR-0: `LlmProvider` trait, error types, and schema validation

## Goal
Define the core `LlmProvider` trait and `LlmError` enum that all three CLI backends
will implement. Add a shared JSON schema validation helper used by the Gemini backend
(PR-2) and optionally by the others for defence-in-depth. No concrete providers yet.

## Non-goals
- No CLI subprocess invocation — that's PR-1 and PR-2.
- No output schema types (`Pass1Output`, `Pass2Output`) — those land in PR-3 once
  the providers are in place and we know what shape the data takes.
- No dispatcher or config wiring — PR-3.
- No prompt construction — Epic 3.

## Success Criteria
- [ ] `LlmError` enum defined with all expected variants
- [ ] `LlmProvider` trait compiles with `async fn analyze()`
- [ ] `AnyProvider` enum stub (no variants yet) compiles
- [ ] `validate_against_schema()` helper passes its unit tests
- [ ] `#![allow(dead_code)]` added (removed in PR-3 when wired up)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Error type (`src/llm/mod.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM process failed to start: {message}")]
    ProcessStart { message: String },
    #[error("LLM process exited with status {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },
    #[error("LLM returned empty output")]
    EmptyOutput,
    #[error("LLM output is not valid JSON: {message}")]
    InvalidJson { message: String },
    #[error("LLM output failed schema validation after {attempts} attempt(s): {errors:?}")]
    ValidationFailed { attempts: u32, errors: Vec<String> },
    #[error("I/O error communicating with LLM process: {0}")]
    Io(#[from] std::io::Error),
}
```

Keep error messages lowercase with no trailing punctuation (Rust convention).

### Trait (`src/llm/mod.rs`)

Use Rust 1.75+ async-fn-in-trait (RPITIT). For object-safe dispatch we use an
`AnyProvider` enum instead of `dyn LlmProvider` (avoids `async-trait` dependency).

```rust
/// Implemented by each CLI backend. `analyze` takes a plaintext prompt and the
/// JSON Schema for the expected response shape; it returns the validated JSON.
pub trait LlmProvider: Send + Sync {
    /// Short identifier shown in error messages and logs (e.g. `"claude"`).
    fn name(&self) -> &str;

    /// Sends `prompt` to the LLM and returns a `serde_json::Value` that
    /// conforms to `schema`. Implementations are responsible for retrying on
    /// validation failure as appropriate for their backend.
    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError>;
}
```

### `AnyProvider` enum (`src/llm/mod.rs`)

Enum-based dispatch avoids `Box<dyn LlmProvider>` and the `async-trait` crate.
PR-1 and PR-2 will add variants; PR-3 implements `LlmProvider for AnyProvider`.

```rust
/// Concrete provider selected from config at startup. Delegates to the
/// appropriate CLI backend without dynamic dispatch.
pub enum AnyProvider {
    Claude(claude::ClaudeProvider),
    Codex(codex::CodexProvider),
    Gemini(gemini::GeminiProvider),
}
```

The `LlmProvider` impl for `AnyProvider` (added in PR-3) simply delegates each
method to the inner type via a `match`.

### Schema validation helper (`src/llm/mod.rs`)

Used by the Gemini backend (schema-less output → validate + retry) and available
to Claude/Codex for defence-in-depth.

The `jsonschema` crate performs JSON Schema draft-7 validation, which matches what
`schemars 0.8` generates.

```rust
/// Validates `value` against `schema`. Returns the list of validation error
/// messages, or an empty Vec if the value is valid.
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> Vec<String> {
    match jsonschema::validator_for(schema) {
        Err(e) => vec![format!("invalid schema: {e}")],
        Ok(validator) => validator
            .iter_errors(value)
            .map(|e| e.to_string())
            .collect(),
    }
}
```

### Tests (`src/llm/mod.rs`)

In a `#[cfg(test)]` block:

- `validate_passes_for_valid_value` — schema requiring a string field, value that
  satisfies it → empty errors Vec
- `validate_fails_for_missing_required_field` — same schema, value missing the
  required field → non-empty errors Vec
- `validate_fails_for_wrong_type` — value with wrong type for a field → error

## Files to Create/Modify
- `src/llm/mod.rs` — replace stub: `LlmError`, `LlmProvider` trait, `AnyProvider`
  enum (variants added in PR-1/2), `validate_against_schema()`, tests
- `Cargo.toml` — add `jsonschema = "0.26"` (or latest 0.x)

## Dependencies
- Depends on: Epic 0 (project scaffold), Epic 1 (no direct dep, but the pattern
  established there — thiserror errors, no `unwrap` in library code — applies here)
- Blocks: PR-1 (Claude/Codex need the trait), PR-2 (Gemini needs validation helper),
  PR-3 (dispatcher implements `LlmProvider for AnyProvider`)

## Open Questions
- Should `analyze()` accept `prompt: &str` directly, or a structured `AnalysisRequest`
  with separate system prompt and user message fields? Leaning toward `&str` for now —
  the prompt construction in Epic 3 can wrap if needed, and the CLIs differ in how
  they separate system/user context.
- Should `validate_against_schema` live in a dedicated `src/llm/validation.rs`
  submodule, or stay in `mod.rs`? Leaning toward `mod.rs` — it's a single function;
  extracting adds file overhead for no clarity gain.
- `jsonschema` crate vs manual validation: `jsonschema` is well-maintained and
  handles edge cases in draft-7 correctly. Manual validation would be fragile. Use
  `jsonschema`.
