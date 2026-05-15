//! Git repository operations via `libgit2` (`git2` crate): repo detection,
//! remote URL parsing, and SHA resolution.

#![allow(dead_code)]

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
}

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

/// Parses a git remote URL and extracts platform, owner, and repo name.
///
/// Supported formats:
/// - HTTPS: `https://github.com/owner/repo[.git]`
/// - SSH SCP-style: `git@github.com:owner/repo[.git]`
/// - SSH protocol: `ssh://[user@]github.com[:port]/owner/repo[.git]`
fn parse_remote_url(url: &str) -> Result<(Platform, String, String), GitError> {
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

    Ok((platform, owner, repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github(owner: &str, repo: &str) -> (Platform, String, String) {
        (Platform::GitHub, owner.into(), repo.into())
    }

    fn bitbucket(owner: &str, repo: &str) -> (Platform, String, String) {
        (Platform::BitBucket, owner.into(), repo.into())
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
        assert_eq!(info.owner, "testowner");
        assert_eq!(info.repo, "testrepo");
        assert_eq!(info.remote_name, "origin");
        assert_eq!(info.remote_url, "https://github.com/testowner/testrepo.git");
        assert!(info.root.exists());
    }
}
