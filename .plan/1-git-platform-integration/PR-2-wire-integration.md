# PR-2: End-to-end wiring — repo detection, PR listing, and diff display

## Goal
Wire the git detection module and GitHub client together in `main.rs`: detect the
repo, load config, fetch open PRs from GitHub, and print them. Add a `--pr <number>`
CLI flag to fetch and display the raw unified diff for a specific PR.

## Non-goals
- No TUI or interactive selection (Epic 5) — output is plain stdout text.
- No diff parsing, splitting, or formatting — raw unified diff output.
- No BitBucket support — prints a clear error if the detected platform is BitBucket.
- No LLM analysis (Epics 2–3).
- No caching (Epic 4).

## Success Criteria
- [ ] Running `easy-diff` in a GitHub repo prints a numbered list of open PRs
- [ ] Running `easy-diff --pr 42` prints the raw unified diff for PR #42
- [ ] Running outside a git repo prints a user-friendly error message
- [ ] Running in a repo with a non-GitHub remote prints a clear "not supported" message
- [ ] Missing GitHub token prints an error directing the user to configure it
- [ ] `main.rs` no longer imports `git2` directly — all git ops go through `crate::git`
- [ ] Existing CLI integration tests in `tests/cli.rs` still pass
- [ ] `#![allow(dead_code)]` removed from `git/mod.rs` and `platform/mod.rs`
      (their public items are now used)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### CLI changes (`src/main.rs`)

Extend the `Cli` struct:

```rust
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool", version)]
struct Cli {
    /// Fetch and display the diff for a specific PR number.
    #[arg(long)]
    pr: Option<u64>,
}
```

### Main flow (`src/main.rs`)

Replace the current body with:

```rust
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // 1. Detect repo
    let repo_info = git::detect_repo_info(".")
        .context("not a git repository — run easy-diff from within a git repo")?;
    tracing::info!(platform = %repo_info.platform, owner = %repo_info.owner,
                   repo = %repo_info.repo, "detected repository");

    // 2. Load config (uses repo root for per-repo config)
    let config = config::load_config(Some(&repo_info.root))?;
    tracing::info!(provider = ?config.llm.default_provider, "resolved LLM provider");

    // 3. Check platform
    if repo_info.platform != Platform::GitHub {
        anyhow::bail!(
            "only GitHub repositories are currently supported (detected: {})",
            repo_info.platform
        );
    }

    // 4. Get token
    let token = config.github.token.ok_or_else(|| {
        anyhow::anyhow!(
            "GitHub token not configured — add [github] token = \"...\" to {}",
            config::global_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/easy-diff/config.toml".into())
        )
    })?;

    // 5. Create client and dispatch
    let client = GithubClient::new(token);

    match cli.pr {
        Some(pr_number) => {
            let diff = client
                .get_pull_request_diff(&repo_info.owner, &repo_info.repo, pr_number)
                .await?;
            println!("{}", diff.diff);
        }
        None => {
            let prs = client
                .list_open_pull_requests(&repo_info.owner, &repo_info.repo)
                .await?;
            print_pr_list(&repo_info, &prs);
        }
    }

    Ok(())
}
```

### Output format

**PR list** (no `--pr` flag):
```
Open pull requests for owner/repo:

  #42  Fix authentication bug             author  2024-01-15
  #41  Add rate limiting support           author  2024-01-14
  #40  Refactor config module              author  2024-01-13
```

A simple `print_pr_list()` helper formats each PR on one line with right-aligned
columns. No color or formatting beyond alignment — keeping it simple and pipeable.

If there are no open PRs, print: `No open pull requests for owner/repo.`

**PR diff** (`--pr N`):
Print `diff.diff` to stdout verbatim. The raw unified diff is pipeable to `delta`,
`bat`, or other diff viewers.

### Removing direct git2 usage

The current `main.rs` does:
```rust
let repo_root = git2::Repository::discover(".")
    .ok()
    .and_then(|r| r.workdir().map(|p| p.to_path_buf()));
```

This is replaced by `git::detect_repo_info(".")` which provides `info.root`. The
`git2` import is removed from `main.rs` — all git operations go through the
`crate::git` module.

### Error handling

All errors bubble up through `anyhow::Result`. Each error site adds `.context()`
with a user-friendly message. The `PlatformError` variants from PR-1 already have
good messages; `anyhow` displays them in the chain.

### Estimated size

This PR is smaller than the 200–400 line target (~100–150 lines) because it's
pure wiring: calling APIs from PR-0 and PR-1, formatting output, and handling
errors. There's no new domain logic. Merging into PR-1 would mix API client
concerns with CLI/UX concerns, making that PR harder to review.

## Files to Create/Modify
- `src/main.rs` — extend CLI struct, replace git2 usage with `crate::git`, wire
  up `GithubClient`, add `print_pr_list()` helper
- `src/git/mod.rs` — remove `#![allow(dead_code)]`
- `src/platform/mod.rs` — remove `#![allow(dead_code)]`

## Dependencies
- Depends on: PR-0 (`detect_repo_info`, `RepoInfo`, `Platform`),
  PR-1 (`GithubClient`, `PullRequest`, `PullRequestDiff`)
- Blocks: Epic 2 (LLM providers plug into this flow), Epic 5 (TUI replaces
  stdout output)

## Open Questions
- Should the PR list include branch names (source → target) in addition to title and
  author? Leaning toward title + author + updated date for now — keeps lines short.
  Branch names can be added with a `--verbose` flag later.
- Should `--pr` accept a PR URL (e.g. `https://github.com/owner/repo/pull/42`) in
  addition to a bare number? Leaning toward number-only — URL parsing adds complexity
  for minimal benefit, and the user already sees the number in the PR list output.
- Should `easy-diff` still work (showing config info, etc.) when run outside a git
  repo, or should it error immediately? Leaning toward erroring — every meaningful
  operation needs a repo, so failing fast is the right behavior.
