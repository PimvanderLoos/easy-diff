# PR-0: Git repository detection and remote URL parsing

## Goal
Detect the git repository from the current working directory using `git2`, read the
`origin` remote, and parse its URL (HTTPS, SSH, and SCP formats) to extract the
platform (GitHub/BitBucket), owner, and repository name.

## Non-goals
- No platform API client — that's PR-1.
- No CLI changes — that's PR-2.
- No support for multiple remotes or configurable remote names — uses `origin` only.
- No BitBucket API support (Epic 6) — but BitBucket URL *detection* is included so
  the parser is complete and we can give a clear "not yet supported" error in PR-2.

## Success Criteria
- [ ] `Platform` enum in `platform/mod.rs` distinguishes GitHub and BitBucket
- [ ] `RepoInfo` struct captures root path, platform, owner, repo name, remote name,
      and remote URL
- [ ] `GitError` enum covers: not a repo, bare repo, no `origin` remote, no URL,
      unparseable URL, unsupported host
- [ ] HTTPS URLs parsed: `https://github.com/owner/repo[.git]`
- [ ] SSH SCP-style URLs parsed: `git@github.com:owner/repo[.git]`
- [ ] SSH protocol URLs parsed: `ssh://git@github.com/owner/repo[.git]`
- [ ] Same three formats parsed for `bitbucket.org`
- [ ] `.git` suffix stripped from repo name
- [ ] Unknown hosts (e.g. `gitlab.com`) produce `GitError::UnsupportedPlatform`
- [ ] Unit tests for all URL formats and error cases
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Platform enum (`src/platform/mod.rs`)

Add to the existing module (which already declares `pub mod github` and `pub mod bitbucket`):

```rust
/// Supported code hosting platforms, identified from remote URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    GitHub,
    BitBucket,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "GitHub"),
            Self::BitBucket => write!(f, "BitBucket"),
        }
    }
}
```

`Platform` lives in the platform module because it identifies which platform applies —
the git module imports it via `crate::platform::Platform`.

### RepoInfo struct (`src/git/mod.rs`)

```rust
/// Information extracted from a git repository's remote URL.
pub struct RepoInfo {
    /// Absolute path to the repository working directory.
    pub root: PathBuf,
    /// Hosting platform detected from the remote URL.
    pub platform: Platform,
    /// Repository owner (user or organization).
    pub owner: String,
    /// Repository name (without `.git` suffix).
    pub repo: String,
    /// Name of the remote that was parsed (always `"origin"` for now).
    pub remote_name: String,
    /// Raw remote URL string as configured in git.
    pub remote_url: String,
}
```

Including `root` so that PR-2 can use `info.root` for config loading, replacing the
current inline `git2::Repository::discover(".")` call in `main.rs`.

### Detection function (`src/git/mod.rs`)

```rust
/// Detects the git repository from `path` and parses the origin remote URL.
pub fn detect_repo_info(path: impl AsRef<Path>) -> Result<RepoInfo, GitError> {
    let repo = Repository::discover(path)
        .map_err(|_| GitError::NotARepository)?;
    let root = repo.workdir()
        .ok_or(GitError::BareRepository)?
        .to_path_buf();
    let remote = repo.find_remote("origin")
        .map_err(|_| GitError::RemoteNotFound { name: "origin".into() })?;
    let url_str = remote.url()
        .ok_or_else(|| GitError::NoRemoteUrl { name: "origin".into() })?;
    let (platform, owner, repo_name) = parse_remote_url(url_str)?;
    Ok(RepoInfo {
        root,
        platform,
        owner,
        repo: repo_name,
        remote_name: "origin".into(),
        remote_url: url_str.to_string(),
    })
}
```

Takes `path` instead of hardcoding `"."` for testability — tests can pass a tempdir.
Uses `git2::Repository::discover()` which walks up the directory tree.

### URL parsing (`src/git/mod.rs`)

A private `parse_remote_url(url: &str) -> Result<(Platform, String, String), GitError>`
function that handles three formats:

1. **HTTPS**: `https://github.com/owner/repo[.git]`
   — Split on `/`, extract host from authority, owner and repo from path segments.

2. **SSH SCP-style**: `git@github.com:owner/repo[.git]`
   — Split on `@` → `host:path`, split on `:` → host and path, split path on `/`.

3. **SSH protocol**: `ssh://[user@]github.com[:port]/owner/repo[.git]`
   — Parse as URL, extract host, ignore port, owner/repo from path.

After extracting the host, map to `Platform`:
- `github.com` → `Platform::GitHub`
- `bitbucket.org` → `Platform::BitBucket`
- anything else → `GitError::UnsupportedPlatform { host }`

Post-processing: strip `.git` suffix and trailing `/` from repo name.
Reject paths with fewer than two segments (need both owner and repo).

### Error type (`src/git/mod.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("not a git repository")]
    NotARepository,
    #[error("bare repositories are not supported")]
    BareRepository,
    #[error("remote '{name}' not found")]
    RemoteNotFound { name: String },
    #[error("remote '{name}' has no URL")]
    NoRemoteUrl { name: String },
    #[error("could not parse remote URL: {url}")]
    UnparseableUrl { url: String },
    #[error("unsupported platform: {host}")]
    UnsupportedPlatform { host: String },
}
```

Note: no `#[from] git2::Error` — we map `git2` errors to our own variants in
`detect_repo_info()` for cleaner error messages. Raw `git2` errors are
implementation details.

### Tests

Unit tests in `src/git/mod.rs` (`#[cfg(test)]` module):

- `parse_github_https` — `https://github.com/owner/repo.git`
- `parse_github_https_no_suffix` — `https://github.com/owner/repo`
- `parse_github_ssh_scp` — `git@github.com:owner/repo.git`
- `parse_github_ssh_scp_no_suffix` — `git@github.com:owner/repo`
- `parse_github_ssh_protocol` — `ssh://git@github.com/owner/repo.git`
- `parse_bitbucket_https` — `https://bitbucket.org/owner/repo.git`
- `parse_bitbucket_ssh_scp` — `git@bitbucket.org:owner/repo.git`
- `parse_unsupported_host` — `https://gitlab.com/owner/repo.git` → `UnsupportedPlatform`
- `parse_malformed_url` — garbage string → `UnparseableUrl`
- `parse_missing_repo_segment` — `https://github.com/owner` → `UnparseableUrl`
- `detect_repo_from_tempdir` — create a git repo via `git2::Repository::init()`,
  add an `origin` remote, call `detect_repo_info()`, verify all fields

### dead_code handling

`detect_repo_info()` won't be called from `main.rs` until PR-2. Add
`#![allow(dead_code)]` to the module (same pattern as `config/mod.rs`).
PR-2 removes the attribute when the function gets wired up.

## Files to Create/Modify
- `src/platform/mod.rs` — add `Platform` enum and `Display` impl (keep existing
  `pub mod github` / `pub mod bitbucket` declarations)
- `src/git/mod.rs` — replace stub with full implementation: `RepoInfo`, `GitError`,
  `detect_repo_info()`, `parse_remote_url()`, unit tests

## Dependencies
- Depends on: Epic 0 (project scaffold — all PRs complete)
- Blocks: PR-1 (GitHub client needs `Platform` from `platform/mod.rs`), PR-2 (wiring
  needs `RepoInfo` and `detect_repo_info()`)

## Open Questions
- Should we support configuring which remote to use (e.g. `upstream` instead of
  `origin`)? Leaning toward hardcoding `origin` — it's the overwhelmingly common case.
  Can add a `remote_name` config field later if needed.
- Should `Platform` have an `Unknown(String)` variant instead of erroring on unsupported
  hosts? Leaning toward erroring — we can't do anything useful with an unknown platform,
  and a clear error is better than a silent no-op. New platforms can be added to the enum.
- Should `detect_repo_info()` take `path: impl AsRef<Path>` or always use `"."`?
  Leaning toward taking a path parameter for testability, with callers passing `"."`.
