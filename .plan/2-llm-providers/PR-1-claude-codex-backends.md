# PR-1: Claude Code CLI and Codex CLI backends

## Goal
Implement `ClaudeProvider` and `CodexProvider` — the two CLI backends whose output
is natively structured (or reliably parseable) without schema-validation retry logic.
Both spawn a subprocess, write the prompt to stdin, read the response from stdout,
and parse the JSON. After this PR the `AnyProvider` enum has its first two working
variants.

## Non-goals
- No Gemini backend — that's PR-2 (needs retry logic not present here).
- No provider dispatch or config wiring — PR-3.
- No output schema types — PR-3.
- No prompt construction — Epic 3.
- No network I/O — all calls go through CLI subprocesses.

## Success Criteria
- [ ] `ClaudeProvider::new(model: Option<String>)` constructs successfully
- [ ] `CodexProvider::new(model: Option<String>)` constructs successfully
- [ ] `AnyProvider` enum updated with `Claude` and `Codex` variants
- [ ] Both providers implement `LlmProvider` (name + analyze)
- [ ] `analyze()` spawns the subprocess, writes prompt to stdin, reads stdout
- [ ] Non-zero exit code or empty stdout → appropriate `LlmError` variant
- [ ] Invalid JSON in stdout → `LlmError::InvalidJson`
- [ ] Unit tests for JSON extraction and error mapping
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Shared subprocess helper (`src/llm/mod.rs`)

Both providers share the same spawn-write-read pattern. Extract a private helper to
avoid duplication:

```rust
/// Spawns `program` with `args`, writes `input` to its stdin, and returns
/// the stdout string. Maps process failures to `LlmError`.
async fn run_subprocess(
    program: &str,
    args: &[&str],
    input: &str,
) -> Result<String, LlmError> {
    use tokio::io::AsyncWriteExt as _;

    let mut child = tokio::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| LlmError::ProcessStart { message: e.to_string() })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(LlmError::ProcessFailed { code, stderr });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if stdout.trim().is_empty() {
        return Err(LlmError::EmptyOutput);
    }

    Ok(stdout)
}
```

Add this private function to `src/llm/mod.rs`. It uses `tokio::process::Command`
(already available via `tokio = { features = ["full"] }`).

### JSON extraction helper (`src/llm/mod.rs`)

LLM CLI output may wrap JSON in markdown fences. Extract the first valid JSON object
or array from the output string:

```rust
/// Extracts the first JSON object or array from `text`, stripping markdown code
/// fences if present. Returns `LlmError::InvalidJson` if none is found.
pub(crate) fn extract_json(text: &str) -> Result<serde_json::Value, LlmError> {
    // Strip ```json ... ``` fences if present
    let stripped = if let Some(inner) = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
    {
        inner.trim_end_matches("```").trim()
    } else {
        text.trim()
    };

    // Find the first `{` or `[` and attempt to parse from there
    let start = stripped
        .find(|c| c == '{' || c == '[')
        .unwrap_or(0);

    serde_json::from_str(&stripped[start..]).map_err(|e| LlmError::InvalidJson {
        message: e.to_string(),
    })
}
```

### `ClaudeProvider` (`src/llm/claude.rs`)

Claude Code CLI (`claude`) reads a prompt from stdin when invoked with `--print`.

```rust
/// Claude Code CLI backend. Invokes the `claude` binary as a subprocess.
pub struct ClaudeProvider {
    /// Model override (e.g. `"claude-opus-4-5"`). `None` uses the CLI default.
    model: Option<String>,
}

impl ClaudeProvider {
    /// Creates a new provider. `model` is forwarded to `--model` if `Some`.
    pub fn new(model: Option<String>) -> Self {
        Self { model }
    }
}

impl LlmProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    async fn analyze(
        &self,
        prompt: &str,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let mut args = vec!["--print", "--output-format", "json"];
        let model_flag;
        if let Some(model) = &self.model {
            model_flag = model.clone();
            args.extend_from_slice(&["--model", &model_flag]);
        }
        let raw = run_subprocess("claude", &args, prompt).await?;
        extract_json(&raw)
    }
}
```

**Note on flags**: `--print` suppresses the interactive REPL; `--output-format json`
requests JSON-wrapped output. Verify these flags against the installed `claude`
version before shipping — they may differ across Claude Code releases.

### `CodexProvider` (`src/llm/codex.rs`)

Codex CLI (`codex`) also reads prompts from stdin and returns plain text or JSON.

```rust
/// Codex CLI backend. Invokes the `codex` binary as a subprocess.
pub struct CodexProvider {
    /// Model override (e.g. `"o4-mini"`). `None` uses the CLI default.
    model: Option<String>,
}

impl CodexProvider {
    pub fn new(model: Option<String>) -> Self {
        Self { model }
    }
}

impl LlmProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    async fn analyze(
        &self,
        prompt: &str,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value, LlmError> {
        let mut args = vec!["--quiet"];
        let model_flag;
        if let Some(model) = &self.model {
            model_flag = model.clone();
            args.extend_from_slice(&["--model", &model_flag]);
        }
        let raw = run_subprocess("codex", &args, prompt).await?;
        extract_json(&raw)
    }
}
```

**Note on flags**: Verify `--quiet` and any JSON output flags against the installed
`codex` binary. The exact invocation may differ from this plan.

### `AnyProvider` update (`src/llm/mod.rs`)

Add variants to the enum stub from PR-0:

```rust
pub enum AnyProvider {
    Claude(claude::ClaudeProvider),
    Codex(codex::CodexProvider),
    Gemini(gemini::GeminiProvider),  // variant added but GeminiProvider is a stub
}
```

`GeminiProvider` stays a struct stub in `gemini.rs` (implemented in PR-2) so the
enum compiles without a circular PR dependency.

### Tests

#### `src/llm/mod.rs` — `extract_json` tests (unit)

- `extract_json_bare_object` — `{"key":"val"}` → parses
- `extract_json_fenced_json` — `\`\`\`json\n{"k":1}\n\`\`\`` → parses
- `extract_json_fenced_no_lang` — `\`\`\`\n{"k":1}\n\`\`\`` → parses
- `extract_json_with_preamble` — `"Here is the output: {\"k\":1}"` → parses by
  seeking first `{`
- `extract_json_invalid` — `"not json at all"` → `Err(LlmError::InvalidJson)`

#### `src/llm/claude.rs` and `src/llm/codex.rs` — provider tests (unit)

Since subprocess invocation requires the actual CLI binary (not available in CI),
the unit tests cover the parts we can test in isolation:

- `provider_name` — `ClaudeProvider::new(None).name() == "claude"`
- `provider_name` — `CodexProvider::new(None).name() == "codex"`

Full subprocess integration tests (requiring actual `claude` / `codex` binaries)
are out of scope for CI and belong in manual/E2E test suites.

## Files to Create/Modify
- `src/llm/mod.rs` — add `run_subprocess()`, `extract_json()`, update `AnyProvider`
  enum, add unit tests
- `src/llm/claude.rs` — replace stub: `ClaudeProvider` struct + `LlmProvider` impl
- `src/llm/codex.rs` — replace stub: `CodexProvider` struct + `LlmProvider` impl
- `src/llm/gemini.rs` — add `GeminiProvider` struct stub (no impl yet, just enough
  to compile as the `Gemini` variant in `AnyProvider`)

## Dependencies
- Depends on: PR-0 (`LlmProvider` trait, `LlmError`, `validate_against_schema`)
- Blocks: PR-3 (dispatcher needs both variants in `AnyProvider`)

## Open Questions
- Should `extract_json` be tested against real Claude/Codex output once available,
  to harden the fence-stripping logic? Yes — add fixture files under `tests/fixtures/`
  in a later pass once real outputs are observed.
- Should `run_subprocess` be in `mod.rs` or in a dedicated `src/llm/subprocess.rs`?
  Leaning toward `mod.rs` — it's one function shared by two files, and a new file
  adds navigation overhead without clarity benefit.
- What happens when the `claude` or `codex` binary is not in `PATH`? The
  `ProcessStart` error variant covers this — the OS returns "No such file or
  directory" which is forwarded in `message`. Epic 5 (CLI MVP) should add a
  preflight check that the configured provider binary exists.
