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
    // Running without a GitHub token configured should exit non-zero and print
    // an actionable error. This test verifies the failure path, not success —
    // end-to-end success requires a live token and network, so it belongs in
    // manual or integration testing rather than unit CI.

    // execute
    let output = binary()
        .env("EASY_DIFF_NO_TOKEN", "1") // ensure no accidental token from env
        .output()
        .expect("failed to spawn binary");

    // verify — must fail with a message about either missing repo or missing token
    assert!(
        !output.status.success(),
        "bare invocation without token must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("token") || stderr.contains("git repository"),
        "error should mention token or git repo; got: {stderr}"
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
