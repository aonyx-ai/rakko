//! The tests of the action of the guide "Run a Tool From an Action"
//!
//! These tests need no taplo, because they check only what the action
//! declares.

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

// region: tools
use otter_check_toml::CheckToml;
use rakko::{Action, RequiredVersion, ToolVersion};
use rakko_tool::MiseFile;

#[test]
fn tools_pins_the_version_of_the_mise_file() {
    let pin = MiseFile::new(include_str!("../mise.rakko.toml")).pinned_version("taplo");

    let tools = CheckToml.tools();

    assert_eq!(
        tools[0].version(),
        &RequiredVersion::Exact(ToolVersion::new(pin))
    );
}
// endregion: tools

#[test]
fn tools_runs_the_program_of_taplo() {
    let tools = CheckToml.tools();

    assert_eq!(tools[0].executable().get(), "taplo");
}
