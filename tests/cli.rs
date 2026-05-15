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
fn no_args_exits_zero() {
    // execute
    let status = binary().status().expect("failed to spawn binary");

    // verify
    assert!(status.success(), "bare invocation must exit 0");
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
