# PR-1: Config module — structs, loading, and merge logic

## Goal
Implement the config module with typed structs for global and per-repo configuration,
TOML deserialization, file loading from standard paths, and merge logic where per-repo
config overrides global config.

## Non-goals
- No custom category config parsing (Epic 7).
- No language-specific rule config (Epic 12).
- No config file creation/wizard — if the file doesn't exist, use defaults.
- No secret redaction in logs (tokens are in config but we only log at debug level).

## Success Criteria
- [ ] `GlobalConfig` struct covers all fields from PLAN.md §11.1
- [ ] `RepoConfig` struct covers the per-repo overrides from PLAN.md §11.2
- [ ] `MergedConfig` (or equivalent) provides a single resolved view
- [ ] Loading from `~/.config/easy-diff/config.toml` works
- [ ] Loading from `.easy-diff/config.toml` (relative to repo root) works
- [ ] Missing config files produce sensible defaults (not errors)
- [ ] Invalid TOML produces a clear, actionable error message
- [ ] Unit tests for merge logic and deserialization edge cases
- [ ] `cargo clippy -- -D warnings` passes

## Technical Approach

### Config structs

```rust
// src/config/mod.rs

/// Top-level resolved configuration, merged from global and per-repo sources.
pub struct Config {
    pub github: GithubConfig,
    pub bitbucket: BitbucketConfig,
    pub llm: LlmConfig,
    pub preferences: Preferences,
}

pub struct GithubConfig {
    pub token: Option<String>,
}

pub struct BitbucketConfig {
    pub username: Option<String>,
    pub app_password: Option<String>,
}

pub struct LlmConfig {
    pub default_provider: Provider,
    pub fallback_provider: Option<Provider>,
}

pub enum Provider {
    Claude,
    Codex,
    Gemini,
}

pub struct Preferences {
    pub context_lines: u32,        // default: 5
    pub max_file_context: u32,     // default: 500
    pub large_pr_threshold: u32,   // default: 5000
}
```

Two internal "raw" structs for deserialization (`RawGlobalConfig`, `RawRepoConfig`) using
`Option<T>` for every field. The merge function walks both raw structs and produces the
resolved `Config`, applying repo overrides on top of global values, falling back to
hardcoded defaults for anything unset.

### Loading logic

1. Determine global config path: `dirs::config_dir()` / `easy-diff/config.toml`
2. Determine repo config path: repo root (from `git2` or CWD) / `.easy-diff/config.toml`
3. Read and deserialize each (if it exists)
4. Merge: repo overrides global, defaults fill gaps

Use `dirs` crate for XDG-compliant config path resolution.

### Error handling

Use `thiserror` for config-specific errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file at {path}")]
    ReadError { path: PathBuf, source: std::io::Error },
    #[error("invalid config at {path}")]
    ParseError { path: PathBuf, source: toml::de::Error },
}
```

### Tests

Unit tests in `src/config/mod.rs`:
- `deserialize_full_global_config` — all fields present
- `deserialize_minimal_config` — only required fields
- `merge_repo_overrides_global` — repo LLM provider overrides global
- `merge_missing_repo_uses_global` — no repo config → global values used
- `merge_all_missing_uses_defaults` — no config files → hardcoded defaults
- `invalid_toml_returns_parse_error`
- `unknown_provider_returns_error`

## Files to Create/Modify
- `Cargo.toml` — add `dirs` dependency
- `src/config/mod.rs` — full implementation replacing stub: structs, loading, merge, errors, tests
- `src/main.rs` — wire up config loading (load config, log resolved provider at info level)

## Dependencies
- Depends on: PR-0 (project structure and stub modules)
- Blocks: all later epics that read config (Epics 1–5+)

## Open Questions
- The `dirs` crate vs `directories` crate — `dirs` is simpler, `directories` is more
  actively maintained and has a richer API. Need to check which is more current.
- Should we validate that the configured LLM CLI binary is actually on `$PATH` at config
  load time, or defer that to when we actually try to invoke it? Leaning toward deferring —
  config loading shouldn't have side effects.
- PLAN.md §11.2 mentions per-repo LLM provider override but not per-repo token overrides.
  Assuming tokens are global-only for now. Flag if this assumption is wrong.
