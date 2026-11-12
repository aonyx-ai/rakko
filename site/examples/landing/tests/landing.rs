//! The tests of the harness on the landing page of the website

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

use std::io;
use std::process::Command;

/// Returns what the harness prints when it runs without a command
///
/// # Errors
///
/// Returns the error of the operating system when the harness does not start.
fn help() -> io::Result<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_otter-landing")).output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn run_without_command_lists_format_rust() {
    let help = help().unwrap();

    assert!(help.contains("format-rust"));
}

#[test]
fn run_without_command_lists_lint_rust() {
    let help = help().unwrap();

    assert!(help.contains("lint-rust"));
}

#[test]
fn run_without_command_lists_test_rust() {
    let help = help().unwrap();

    assert!(help.contains("test-rust"));
}
