//! Git repository operations via `libgit2` (`git2` crate): repo detection,
//! remote URL parsing, and SHA resolution.

use std::path::{Path, PathBuf};

use git2::Repository;
use thiserror::Error;

use crate::platform::Platform;

/// Errors produced by git repository detection and URL parsing.
#[derive(Debug, Error)]
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
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("commit not found: {sha}")]
    CommitNotFound { sha: String },
}

/// Information extracted from a git repository's remote URL.
pub struct RepoInfo {
    /// Absolute path to the repository working directory.
    pub root: PathBuf,
    /// Hosting platform detected from the remote URL.
    pub platform: Platform,
    /// Hostname of the remote (e.g. `"github.com"`, `"github.example.com"`).
    pub host: String,
    /// Repository owner (user or organization).
    pub owner: String,
    /// Repository name (without `.git` suffix).
    pub repo: String,
    /// Name of the remote that was parsed (always `"origin"` for now).
    #[allow(dead_code)]
    pub remote_name: String,
    /// Raw remote URL string as configured in git.
    #[allow(dead_code)]
    pub remote_url: String,
}

/// Detects the git repository from `path` and parses the origin remote URL.
pub fn detect_repo_info(path: impl AsRef<Path>) -> Result<RepoInfo, GitError> {
    let repo = Repository::discover(path).map_err(|_| GitError::NotARepository)?;
    let root = repo
        .workdir()
        .ok_or(GitError::BareRepository)?
        .to_path_buf();
    let remote = repo
        .find_remote("origin")
        .map_err(|_| GitError::RemoteNotFound {
            name: "origin".into(),
        })?;
    let url_str = remote.url().ok_or_else(|| GitError::NoRemoteUrl {
        name: "origin".into(),
    })?;
    let (platform, host, owner, repo_name) = parse_remote_url(url_str)?;
    Ok(RepoInfo {
        root,
        platform,
        host,
        owner,
        repo: repo_name,
        remote_name: "origin".into(),
        remote_url: url_str.to_string(),
    })
}

/// Parses a git remote URL and extracts platform, host, owner, and repo name.
///
/// Supported formats:
/// - HTTPS: `https://github.com/owner/repo[.git]`
/// - SSH SCP-style: `git@github.com:owner/repo[.git]`
/// - SSH protocol: `ssh://[user@]github.com[:port]/owner/repo[.git]`
fn parse_remote_url(url: &str) -> Result<(Platform, String, String, String), GitError> {
    let unparseable = || GitError::UnparseableUrl {
        url: url.to_string(),
    };

    let (host, path) = if url.starts_with("https://") || url.starts_with("http://") {
        // HTTPS: https://github.com/owner/repo.git
        let without_scheme = url.split_once("://").map(|x| x.1).ok_or_else(unparseable)?;
        let slash = without_scheme.find('/').ok_or_else(unparseable)?;
        let host = &without_scheme[..slash];
        let path = &without_scheme[slash + 1..];
        (host.to_string(), path.to_string())
    } else if url.starts_with("ssh://") {
        // SSH protocol: ssh://git@github.com/owner/repo.git
        let without_scheme = url.split_once("://").map(|x| x.1).ok_or_else(unparseable)?;
        // Strip optional user@
        let host_and_path = if let Some(at) = without_scheme.find('@') {
            &without_scheme[at + 1..]
        } else {
            without_scheme
        };
        // Strip optional :port
        let slash = host_and_path.find('/').ok_or_else(unparseable)?;
        let host_part = &host_and_path[..slash];
        let host = if let Some(colon) = host_part.find(':') {
            &host_part[..colon]
        } else {
            host_part
        };
        let path = &host_and_path[slash + 1..];
        (host.to_string(), path.to_string())
    } else if url.contains('@') && url.contains(':') {
        // SSH SCP-style: git@github.com:owner/repo.git
        let after_at = url.split_once('@').map(|x| x.1).ok_or_else(unparseable)?;
        let colon = after_at.find(':').ok_or_else(unparseable)?;
        let host = &after_at[..colon];
        let path = &after_at[colon + 1..];
        (host.to_string(), path.to_string())
    } else {
        return Err(unparseable());
    };

    let platform = match host.as_str() {
        "github.com" => Platform::GitHub,
        "bitbucket.org" => Platform::BitBucket,
        other => {
            return Err(GitError::UnsupportedPlatform {
                host: other.to_string(),
            })
        }
    };

    // Parse owner/repo from path, stripping leading slashes and .git suffix
    let path = path.trim_start_matches('/').trim_end_matches('/');
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(unparseable());
    }
    let owner = parts[0].to_string();
    let repo = parts[1]
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string();
    if repo.is_empty() {
        return Err(unparseable());
    }

    Ok((platform, host, owner, repo))
}

/// Returns the list of file paths that changed between two commits.
///
/// Uses `git2` to compute a diff between the two tree objects and extract
/// changed paths. Both `old_sha` and `new_sha` must refer to commits (or
/// objects that peel to commits) present in the repository at `repo_path`.
///
/// Renamed files are reported as both the old path (deleted) and the new path
/// (added), matching the behaviour of `git diff --name-only`.
pub fn changed_files_between(
    repo_path: impl AsRef<Path>,
    old_sha: &str,
    new_sha: &str,
) -> Result<Vec<String>, GitError> {
    let repo = Repository::discover(repo_path).map_err(|_| GitError::NotARepository)?;

    let old_oid = git2::Oid::from_str(old_sha).map_err(|_| GitError::CommitNotFound {
        sha: old_sha.to_owned(),
    })?;
    let new_oid = git2::Oid::from_str(new_sha).map_err(|_| GitError::CommitNotFound {
        sha: new_sha.to_owned(),
    })?;

    let old_commit = repo
        .find_commit(old_oid)
        .map_err(|_| GitError::CommitNotFound {
            sha: old_sha.to_owned(),
        })?;
    let new_commit = repo
        .find_commit(new_oid)
        .map_err(|_| GitError::CommitNotFound {
            sha: new_sha.to_owned(),
        })?;

    let old_tree = old_commit.tree()?;
    let new_tree = new_commit.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

    let mut paths: Vec<String> = Vec::new();
    for delta in diff.deltas() {
        if let Some(path) = delta.new_file().path() {
            if let Some(s) = path.to_str() {
                if !paths.contains(&s.to_owned()) {
                    paths.push(s.to_owned());
                }
            }
        }
        // For renames/deletes, also include the old path.
        if let Some(old_path) = delta.old_file().path() {
            if let Some(s) = old_path.to_str() {
                if !paths.contains(&s.to_owned()) {
                    paths.push(s.to_owned());
                }
            }
        }
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github(owner: &str, repo: &str) -> (Platform, String, String, String) {
        (
            Platform::GitHub,
            "github.com".into(),
            owner.into(),
            repo.into(),
        )
    }

    fn bitbucket(owner: &str, repo: &str) -> (Platform, String, String, String) {
        (
            Platform::BitBucket,
            "bitbucket.org".into(),
            owner.into(),
            repo.into(),
        )
    }

    #[test]
    fn parse_github_https() {
        assert_eq!(
            parse_remote_url("https://github.com/owner/repo.git").unwrap(),
            github("owner", "repo")
        );
    }

    #[test]
    fn parse_github_https_no_suffix() {
        assert_eq!(
            parse_remote_url("https://github.com/owner/repo").unwrap(),
            github("owner", "repo")
        );
    }

    #[test]
    fn parse_github_ssh_scp() {
        assert_eq!(
            parse_remote_url("git@github.com:owner/repo.git").unwrap(),
            github("owner", "repo")
        );
    }

    #[test]
    fn parse_github_ssh_scp_no_suffix() {
        assert_eq!(
            parse_remote_url("git@github.com:owner/repo").unwrap(),
            github("owner", "repo")
        );
    }

    #[test]
    fn parse_github_ssh_protocol() {
        assert_eq!(
            parse_remote_url("ssh://git@github.com/owner/repo.git").unwrap(),
            github("owner", "repo")
        );
    }

    #[test]
    fn parse_bitbucket_https() {
        assert_eq!(
            parse_remote_url("https://bitbucket.org/owner/repo.git").unwrap(),
            bitbucket("owner", "repo")
        );
    }

    #[test]
    fn parse_bitbucket_ssh_scp() {
        assert_eq!(
            parse_remote_url("git@bitbucket.org:owner/repo.git").unwrap(),
            bitbucket("owner", "repo")
        );
    }

    #[test]
    fn parse_unsupported_host() {
        let err = parse_remote_url("https://gitlab.com/owner/repo.git").unwrap_err();
        assert!(matches!(err, GitError::UnsupportedPlatform { .. }));
    }

    #[test]
    fn parse_malformed_url() {
        let err = parse_remote_url("not-a-url-at-all").unwrap_err();
        assert!(matches!(err, GitError::UnparseableUrl { .. }));
    }

    #[test]
    fn parse_missing_repo_segment() {
        let err = parse_remote_url("https://github.com/owner").unwrap_err();
        assert!(matches!(err, GitError::UnparseableUrl { .. }));
    }

    #[test]
    fn detect_repo_from_tempdir() {
        use git2::Repository;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://github.com/testowner/testrepo.git")
            .unwrap();

        let info = detect_repo_info(dir.path()).unwrap();
        assert_eq!(info.platform, Platform::GitHub);
        assert_eq!(info.host, "github.com");
        assert_eq!(info.owner, "testowner");
        assert_eq!(info.repo, "testrepo");
        assert_eq!(info.remote_name, "origin");
        assert_eq!(info.remote_url, "https://github.com/testowner/testrepo.git");
        assert!(info.root.exists());
    }

    // ── changed_files_between tests ──────────────────────────────────────────

    /// Creates a minimal git repo with two commits and returns (repo_dir, old_sha, new_sha).
    /// Commit 1 adds "a.rs". Commit 2 adds "b.rs" and modifies "a.rs".
    fn make_two_commit_repo() -> (tempfile::TempDir, String, String) {
        use git2::{Repository, Signature};
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let sig = Signature::now("Test", "test@example.com").unwrap();

        // Commit 1: add a.rs
        fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("a.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let oid1 = repo
            .commit(Some("HEAD"), &sig, &sig, "first commit", &tree, &[])
            .unwrap();

        // Commit 2: modify a.rs, add b.rs
        fs::write(dir.path().join("a.rs"), "fn a() { /* updated */ }").unwrap();
        fs::write(dir.path().join("b.rs"), "fn b() {}").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("a.rs")).unwrap();
        index.add_path(std::path::Path::new("b.rs")).unwrap();
        index.write().unwrap();
        let tree_oid2 = index.write_tree().unwrap();
        let tree2 = repo.find_tree(tree_oid2).unwrap();
        let parent = repo.find_commit(oid1).unwrap();
        let oid2 = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "second commit",
                &tree2,
                &[&parent],
            )
            .unwrap();

        (dir, oid1.to_string(), oid2.to_string())
    }

    #[test]
    fn changed_files_between_detects_modified_and_added_files() {
        // setup
        let (dir, old_sha, new_sha) = make_two_commit_repo();

        // execute
        let files = changed_files_between(dir.path(), &old_sha, &new_sha).unwrap();

        // verify — both a.rs (modified) and b.rs (added) should appear
        assert!(
            files.contains(&"a.rs".to_owned()),
            "a.rs was modified and should appear"
        );
        assert!(
            files.contains(&"b.rs".to_owned()),
            "b.rs was added and should appear"
        );
    }

    #[test]
    fn changed_files_between_returns_empty_for_identical_commits() {
        // setup
        let (dir, sha, _) = make_two_commit_repo();

        // execute — diff a commit with itself
        let files = changed_files_between(dir.path(), &sha, &sha).unwrap();

        // verify
        assert!(
            files.is_empty(),
            "diffing a commit with itself should return no files"
        );
    }

    #[test]
    fn changed_files_between_errors_on_unknown_sha() {
        // setup
        let (dir, _, _) = make_two_commit_repo();
        let bad_sha = "0000000000000000000000000000000000000000";

        // execute
        let result = changed_files_between(dir.path(), bad_sha, bad_sha);

        // verify
        assert!(result.is_err(), "unknown SHA should produce an error");
    }
}
