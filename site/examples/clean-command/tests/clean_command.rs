//! The test of the harness of the guide "Add a Command"

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

use std::process::Command;

#[test]
fn run_without_command_lists_the_command_beside_the_actions() {
    let output = Command::new(env!("CARGO_BIN_EXE_otter-clean-command"))
        .output()
        .unwrap();

    assert!(String::from_utf8_lossy(&output.stdout).contains("Removes the build output"));
}
