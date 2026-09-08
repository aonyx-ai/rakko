//! Tests that drive the crate the way a harness does
//!
//! A harness sees the crate from outside, and a written command that drives
//! actions reaches for the report that a run of one action renders. These
//! tests therefore live beside the crate instead of inside it: a report that
//! only the crate itself can build would fail them.

// An assertion in a test panics by design. A `# Panics` section on every test
// would repeat that and give the reader no information.
#![allow(clippy::missing_panics_doc)]

use rakko_action::{FilePath, Finding, Location, Outcome, Summary, action_name};
use rakko_cli::Report;

// cli[verify report.written]
#[test]
fn report_of_a_failed_action_shows_what_it_found() {
    let location = Location::File {
        path: FilePath::try_from("deny.toml").expect("the test names a file of the project"),
    };
    let finding = Finding::builder()
        .message("the file is not formatted")
        .location(location)
        .build();
    let outcome = Outcome::Failed {
        findings: vec![finding],
        repairs: Vec::new(),
    };

    let report = Report::new(action_name!("format-toml"), outcome);

    assert_eq!(
        report.to_string(),
        "deny.toml: the file is not formatted\nformat-toml: 1 finding"
    );
}

// cli[verify report.written]
#[test]
fn report_of_a_passing_action_shows_the_action_and_its_summary() {
    let outcome = Outcome::Passed {
        summary: Some(Summary::new("checked 3 files")),
    };

    let report = Report::new(action_name!("format-toml"), outcome);

    assert_eq!(report.to_string(), "format-toml: passed, checked 3 files");
}
