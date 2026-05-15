# PR-2: GitHub Actions CI pipeline

## Goal
Add a GitHub Actions workflow that runs build, test, clippy, and fmt checks on every
push and pull request, ensuring code quality gates are enforced from the start.

## Non-goals
- No release/publish workflow.
- No cross-compilation or multi-platform matrix (can add later).
- No code coverage reporting.
- No caching of Cargo build artifacts (can optimize later if CI is slow).

## Success Criteria
- [ ] `.github/workflows/ci.yml` exists and is valid YAML
- [ ] Workflow triggers on push to any branch and on pull requests
- [ ] Workflow runs four jobs: build, test, clippy (deny warnings), fmt check
- [ ] All four jobs pass on the current codebase
- [ ] Workflow uses a recent stable Rust toolchain

## Technical Approach

Single workflow file with four steps in one job (to share the compilation cache within
the job). Using `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, and
`Swatinem/rust-cache@v2` for faster builds.

```yaml
name: CI

on:
  push:
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Build & Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - name: Build
        run: cargo build --all-targets
      - name: Test
        run: cargo test
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Format
        run: cargo fmt --check
```

Using a single job with sequential steps (rather than parallel jobs) because:
1. All steps share the same compilation output — parallel jobs would each compile from scratch.
2. The codebase is small; total CI time will be well under 5 minutes.
3. If build fails, the later steps are skipped automatically.

### System dependencies

`git2` (libgit2) and `rusqlite` (SQLite) need system libraries. `rusqlite` with the
`bundled` feature compiles SQLite from source, so no system dep needed. `git2` may need
`libssl-dev` and `pkg-config` on Ubuntu — add an apt install step if needed.

## Files to Create/Modify
- `.github/workflows/ci.yml` — new workflow file

## Dependencies
- Depends on: PR-0 (need a compilable project), PR-1 (config module adds a dep and code)
- Blocks: nothing directly, but all future PRs benefit from CI

## Open Questions
- Should we add `cargo audit` to CI now or later? Leaning toward later — it's useful but
  not essential at this stage, and it adds a tool installation step.
- Should `Swatinem/rust-cache` be pinned to a specific version or use `@v2`? Using `@v2`
  for now since it tracks the latest v2.x. Can pin later if reproducibility matters.
