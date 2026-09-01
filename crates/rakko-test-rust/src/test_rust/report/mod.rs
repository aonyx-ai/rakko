//! What nextest reported about a run
//!
//! Nextest writes its structured report as one JSON document per line on
//! its standard output, next to the JSON of cargo, and this module reads the
//! lines of nextest. The shape of the lines belongs to a version of nextest,
//! and this module is the one place that knows it.

/// One test that failed
mod failure;
/// Where and why a test panicked
mod panic;

use getset::{CopyGetters, Getters};
use serde::Deserialize;

pub use self::failure::TestFailure;
pub use self::panic::Panic;

/// The kind of a line that reports one test
const TEST: &str = "test";

/// The kind of a line that reports one binary of tests
const SUITE: &str = "suite";

/// The event of a test that failed
const FAILED: &str = "failed";

/// The event of a binary whose tests all passed
const OK: &str = "ok";

/// What nextest reported about one run
///
/// The report holds every test that failed, with what the test wrote, and
/// the count of the tests that ran. A caller that ran nextest with the
/// structured report reads its standard output into a report and decides
/// what the failures mean for its outcome.
///
/// # Examples
///
/// ```
/// use rakko_test_rust::NextestReport;
///
/// let stdout = r#"{"type":"suite","event":"ok","passed":3,"failed":0,"ignored":1}"#;
///
/// let report = NextestReport::read(stdout);
///
/// assert_eq!(report.ran(), 3);
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct NextestReport {
    /// The tests that failed, in the order of the output
    #[getset(get = "pub")]
    failures: Vec<TestFailure>,

    /// How many tests ran, which is every test that passed or failed
    #[getset(get_copy = "pub")]
    ran: u64,
}

impl NextestReport {
    /// Reads the report of a run from what nextest wrote to its standard
    /// output
    ///
    /// The reading keeps every test that failed and sums the tests of every
    /// binary that finished. It ignores every other line: a test that
    /// started or passed, the lines of cargo, and a line that is not JSON at
    /// all, so the two tools can share the stream.
    // testrust[impl run.failed]
    // testrust[impl run.none]
    // testrust[impl run.passed]
    pub fn read(stdout: &str) -> Self {
        let mut failures = Vec::new();
        let mut ran = 0;

        for line in stdout.lines() {
            let Ok(record) = serde_json::from_str::<Record>(line) else {
                continue;
            };

            match (record.kind.as_str(), record.event.as_str()) {
                (TEST, FAILED) => {
                    if let Some(name) = record.name {
                        failures.push(TestFailure::new(name, record.stdout.unwrap_or_default()));
                    }
                }
                (SUITE, OK | FAILED) => {
                    ran += record.passed.unwrap_or_default() + record.failed.unwrap_or_default();
                }
                _ => {}
            }
        }

        Self { failures, ran }
    }
}

/// One line of the structured report of nextest
///
/// Nextest writes more than these fields, and the reading ignores the rest,
/// so a field that a new version adds does not break it.
#[derive(Deserialize)]
struct Record {
    /// Whether the line is about one test or about one binary of tests
    #[serde(rename = "type")]
    kind: String,

    /// What happened
    event: String,

    /// The name of the test, when the line is about one
    name: Option<String>,

    /// What the test wrote, when it failed
    stdout: Option<String>,

    /// How many tests of the binary passed, when the line closes a binary
    passed: Option<u64>,

    /// How many tests of the binary failed, when the line closes a binary
    failed: Option<u64>,
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A line of cargo that shares the stream
    const ARTIFACT: &str = r#"{"reason":"compiler-artifact","target":{"name":"probe"}}"#;

    /// The line of a test that failed
    const FAILED_TEST: &str = r#"{"type":"test","event":"failed","name":"probe::suite$fails","exec_time":0.009,"stdout":"\nthread 'fails' panicked at tests/suite.rs:8:5:\nthe probe fails on purpose\n"}"#;

    /// The line that closes a binary with a failure
    const FAILED_SUITE: &str = r#"{"type":"suite","event":"failed","passed":1,"failed":1,"ignored":0,"measured":0,"filtered_out":0,"exec_time":0.018}"#;

    /// The line of a test that passed
    const PASSED_TEST: &str =
        r#"{"type":"test","event":"ok","name":"probe::suite$passes","exec_time":0.009}"#;

    /// The line that closes a binary whose tests passed
    const PASSED_SUITE: &str = r#"{"type":"suite","event":"ok","passed":2,"failed":0,"ignored":1,"measured":0,"filtered_out":0,"exec_time":0.01}"#;

    /// The line of a test that started
    const STARTED_TEST: &str = r#"{"type":"test","event":"started","name":"probe::suite$passes"}"#;

    /// Returns the output of a run made of the given lines
    fn output(lines: &[&str]) -> String {
        let mut output = lines.join("\n");
        output.push('\n');

        output
    }

    // testrust[verify run.failed]
    #[test]
    fn read_a_failed_test_keeps_its_name() {
        let report = NextestReport::read(&output(&[FAILED_TEST, FAILED_SUITE]));

        assert_eq!(
            report.failures().first().map(TestFailure::name),
            Some(&"probe::suite$fails".to_owned())
        );
    }

    // testrust[verify run.failed]
    #[test]
    fn read_a_failed_test_keeps_what_it_wrote() {
        let report = NextestReport::read(&output(&[FAILED_TEST, FAILED_SUITE]));

        assert!(
            report
                .failures()
                .first()
                .is_some_and(|failure| failure.output().contains("the probe fails on purpose")),
            "expected the output of the test, got {:?}",
            report.failures()
        );
    }

    // testrust[verify run.passed]
    #[test]
    fn read_a_line_of_cargo_ignores_it() {
        let report = NextestReport::read(&output(&[ARTIFACT, PASSED_SUITE]));

        assert!(report.failures().is_empty());
    }

    // testrust[verify run.passed]
    #[test]
    fn read_a_passed_test_keeps_no_failure() {
        let report = NextestReport::read(&output(&[STARTED_TEST, PASSED_TEST, PASSED_SUITE]));

        assert!(report.failures().is_empty());
    }

    // testrust[verify run.none]
    #[test]
    fn read_an_empty_report_ran_nothing() {
        let report = NextestReport::read("");

        assert_eq!(report.ran(), 0);
    }

    // testrust[verify run.passed]
    #[test]
    fn read_sums_the_tests_that_passed_and_failed_over_the_binaries() {
        let report = NextestReport::read(&output(&[PASSED_SUITE, FAILED_TEST, FAILED_SUITE]));

        assert_eq!(report.ran(), 4);
    }
}
