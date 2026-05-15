# PR-2: Gemini CLI backend with schema validation and retry

## Goal
Implement `GeminiProvider` — the third CLI backend. Unlike Claude and Codex, the
Gemini CLI does not guarantee JSON-conformant structured output, so `analyze()` must
validate the parsed response against the caller-supplied schema and retry on failure,
up to a configurable maximum.

## Non-goals
- No changes to Claude or Codex providers — those are complete after PR-1.
- No provider dispatch or config wiring — PR-3.
- No output schema types — PR-3.
- No prompt construction — Epic 3.

## Success Criteria
- [ ] `GeminiProvider::new(model, max_retries)` constructs successfully
- [ ] `AnyProvider::Gemini` variant is fully implemented (not a stub)
- [ ] `analyze()` retries up to `max_retries` times on schema validation failure
- [ ] Each retry appends the validation errors to the prompt as correction context
- [ ] Exceeding `max_retries` → `LlmError::ValidationFailed { attempts, errors }`
- [ ] Non-zero exit or empty output → appropriate `LlmError` (no retry for process errors)
- [ ] Unit tests for retry logic using a mock closure
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Why Gemini needs retry

Gemini CLI does not expose a `--output-format json` or schema-enforcement flag at
the subprocess level. The model may respond with prose, partial JSON, or JSON that
doesn't match the schema. The retry loop:

1. Calls the subprocess.
2. Extracts JSON from the response (using `extract_json` from PR-1).
3. Validates against `schema` using `validate_against_schema` from PR-0.
4. On failure, appends the errors to the prompt as a correction instruction and
   retries.

After `max_retries` total attempts, returns `LlmError::ValidationFailed`.

### `GeminiProvider` struct (`src/llm/gemini.rs`)

```rust
/// Gemini CLI backend. Invokes the `gemini` binary as a subprocess and retries
/// up to `max_retries` times on schema validation failure.
pub struct GeminiProvider {
    /// Model override (e.g. `"gemini-2.0-flash"`). `None` uses the CLI default.
    model: Option<String>,
    /// Maximum total attempts (including the first). Default: 3.
    max_retries: u32,
}

impl GeminiProvider {
    /// Creates a new provider.
    ///
    /// `max_retries` must be at least 1. If 0 is passed it is silently clamped
    /// to 1 so the first attempt is always made.
    pub fn new(model: Option<String>, max_retries: u32) -> Self {
        Self {
            model,
            max_retries: max_retries.max(1),
        }
    }
}
```

### Retry loop (`src/llm/gemini.rs`)

```rust
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let mut current_prompt = prompt.to_string();
        let mut last_errors: Vec<String> = Vec::new();

        for attempt in 1..=self.max_retries {
            let mut args = vec![];
            let model_flag;
            if let Some(model) = &self.model {
                model_flag = model.clone();
                args.extend_from_slice(&["--model", &model_flag]);
            }

            let raw = run_subprocess("gemini", &args, &current_prompt).await?;
            let value = match extract_json(&raw) {
                Ok(v) => v,
                Err(e) => {
                    last_errors = vec![e.to_string()];
                    if attempt < self.max_retries {
                        current_prompt = build_retry_prompt(prompt, &last_errors);
                    }
                    continue;
                }
            };

            let errors = validate_against_schema(&value, schema);
            if errors.is_empty() {
                return Ok(value);
            }

            last_errors = errors;
            if attempt < self.max_retries {
                current_prompt = build_retry_prompt(prompt, &last_errors);
            }
        }

        Err(LlmError::ValidationFailed {
            attempts: self.max_retries,
            errors: last_errors,
        })
    }
}
```

### Retry prompt construction (`src/llm/gemini.rs`)

```rust
/// Appends a correction instruction to the original prompt, listing the
/// schema validation errors from the previous attempt.
fn build_retry_prompt(original: &str, errors: &[String]) -> String {
    let error_list = errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("  {}. {e}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{original}\n\n\
        Your previous response did not conform to the required JSON schema. \
        Please correct the following errors and respond with valid JSON only:\n\
        {error_list}"
    )
}
```

### `AnyProvider` — complete the Gemini variant (`src/llm/mod.rs`)

The `GeminiProvider` stub from PR-1 is now a full implementation. No change to the
`AnyProvider` enum definition itself — the `Gemini(GeminiProvider)` variant was
already declared.

Add the `LlmProvider` implementation for `AnyProvider` in `src/llm/mod.rs`
(it can be added here since all three concrete types are now fully implemented):

```rust
impl LlmProvider for AnyProvider {
    fn name(&self) -> &str {
        match self {
            Self::Claude(p) => p.name(),
            Self::Codex(p) => p.name(),
            Self::Gemini(p) => p.name(),
        }
    }

    async fn analyze(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        match self {
            Self::Claude(p) => p.analyze(prompt, schema).await,
            Self::Codex(p) => p.analyze(prompt, schema).await,
            Self::Gemini(p) => p.analyze(prompt, schema).await,
        }
    }
}
```

### Tests (`src/llm/gemini.rs`)

Testing the retry logic without spawning a real subprocess requires isolating the
logic. Extract the inner retry logic into a testable helper:

```rust
/// Core retry loop, parameterized over the fetch function. Allows unit testing
/// without spawning subprocesses.
async fn retry_loop<F, Fut>(
    prompt: &str,
    schema: &serde_json::Value,
    max_retries: u32,
    fetch: F,
) -> Result<serde_json::Value, LlmError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<String, LlmError>>,
{
    // ... (same logic as `analyze`, but calls `fetch(current_prompt)` instead
    // of `run_subprocess`)
}
```

`analyze` delegates to `retry_loop` with `run_subprocess` as the fetch function.

Unit tests in `#[cfg(test)]`:

- `retry_loop_succeeds_on_first_attempt` — fetch returns valid JSON matching a
  simple schema → `Ok(value)`, no retries
- `retry_loop_retries_on_invalid_json` — fetch returns `"not json"` for the first
  two attempts, valid JSON on the third → `Ok(value)`
- `retry_loop_retries_on_schema_violation` — fetch returns `{"wrong_field": 1}`
  for the first two attempts, correct JSON on the third → `Ok(value)`
- `retry_loop_exhausts_retries` — fetch always returns schema-invalid JSON →
  `Err(LlmError::ValidationFailed { attempts: 3, .. })`
- `retry_loop_stops_on_process_error` — fetch returns `Err(LlmError::ProcessFailed)`
  immediately, no retry → propagates error unchanged
- `build_retry_prompt_includes_errors` — verify error list is appended to prompt

## Files to Create/Modify
- `src/llm/gemini.rs` — replace stub: `GeminiProvider`, `retry_loop`, `build_retry_prompt`,
  `LlmProvider` impl, tests
- `src/llm/mod.rs` — add `LlmProvider for AnyProvider` impl

## Dependencies
- Depends on: PR-0 (`LlmError`, `validate_against_schema`), PR-1 (`run_subprocess`,
  `extract_json`, `GeminiProvider` struct stub in `AnyProvider`)
- Blocks: PR-3 (dispatcher and config wiring require all three providers to be complete)

## Open Questions
- Should `retry_loop` use a `tokio::time::sleep` between retries (exponential backoff)?
  Leaning toward no for now — rate limits aren't a concern for local CLI subprocesses,
  and a delay would slow down tests. Can add later if needed.
- Should the validation error list be truncated before appending to the retry prompt
  (to avoid blowing up the context window with many errors)? Leaning toward keeping
  the first 5 errors only. Add a `const MAX_ERRORS_IN_RETRY: usize = 5` guard.
- Should `GeminiProvider` accept a `system_prompt: Option<String>` separate from the
  user prompt, for CLIs that distinguish the two? Leaning toward a single `prompt`
  string for now, consistent with Claude and Codex — Epic 3 will construct the prompt
  and can concatenate system + user context.
