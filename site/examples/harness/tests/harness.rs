//! The test of the harness of the tutorial "Get Started"

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

use std::process::Command;

#[test]
fn run_without_command_lists_the_three_actions() {
    let output = Command::new(env!("CARGO_BIN_EXE_otter-harness"))
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&output.stdout);
    let actions = ["format-rust", "lint-rust", "test-rust"];

    assert!(actions.iter().all(|action| help.contains(action)));
}
