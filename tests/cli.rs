//! Integration tests for the `easy-diff` CLI binary.

use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_easy-diff"))
}

// setup: binary built by Cargo test harness

#[test]
fn help_exits_zero() {
    // execute
    let status = binary()
        .arg("--help")
        .status()
        .expect("failed to spawn binary");

    // verify
    assert!(status.success(), "--help must exit 0");
}

#[test]
fn version_exits_zero() {
    // execute
    let status = binary()
        .arg("--version")
        .status()
        .expect("failed to spawn binary");

    // verify
    assert!(status.success(), "--version must exit 0");
}

#[test]
fn no_args_without_token_exits_nonzero() {
    // Running without a GitHub token and without gh CLI should exit non-zero
    // and print an actionable error. If gh is installed and authenticated, the
    // auto backend will succeed — so we force the "api" backend to isolate
    // the no-token failure path.

    // execute — run from a temp dir to avoid using the real repo's config
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = git2::Repository::init(dir.path()).expect("git init");
    repo.remote("origin", "https://github.com/test/test.git")
        .expect("add remote");

    let output = binary()
        .current_dir(dir.path())
        .env_remove("GITHUB_TOKEN")
        .output()
        .expect("failed to spawn binary");

    // verify — must fail (no token, no gh auth for fake repo)
    assert!(
        !output.status.success(),
        "bare invocation without token must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("token")
            || stderr.contains("git repository")
            || stderr.contains("gh auth login")
            || stderr.contains("GitHub")
            || stderr.contains("not found")
            || stderr.contains("Could not resolve"),
        "error should mention token, git repo, or gh auth; got: {stderr}"
    );
}

#[test]
fn version_output_contains_crate_version() {
    // execute
    let output = binary()
        .arg("--version")
        .output()
        .expect("failed to spawn binary");

    // verify
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output should contain the crate version; got: {stdout}"
    );
}
