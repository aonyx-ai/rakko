//! The tests of the action of the guide "Run a Tool From an Action"
//!
//! These tests need no taplo, because they check only what the action
//! declares.

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

// region: tools
use otter_check_toml::CheckToml;
use rakko::Action;

#[test]
fn tools_pin_the_version_of_the_mise_file() {
    let tools = CheckToml.tools();

    let version = tools[0].version();

    assert!(
        include_str!("../mise.rakko.toml").contains(&format!("taplo = \"{version}\"")),
        "expected the version of the mise file, got {version}"
    );
}
// endregion: tools

#[test]
fn tools_with_one_pin_hold_taplo_alone() {
    let tools = CheckToml.tools();

    let names: Vec<&str> = tools.iter().map(|tool| tool.tool().get()).collect();

    assert_eq!(names, ["taplo"]);
}
