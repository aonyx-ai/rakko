//! What the test harness reported about the examples of a workspace
//!
//! The harness of Rust writes its report as text: one line per example, one
//! block for each example that failed, and a summary that counts what ran.
//! Cargo writes its own records as JSON to the same stream, so the report is
//! a set of lines among many, and this module picks them out and reads them.
//! The shape belongs to a version of the harness, and keeping the reading in
//! one place keeps the version surface in one place as well.

/// The error that stops the reading
mod error;
/// One example that failed
mod failure;

use getset::{CopyGetters, Getters};

pub use self::error::ReadDoctestReportError;
pub use self::failure::DoctestFailure;

/// The line that the harness writes above the blocks of the failed examples,
/// and again above the list of their names
const FAILURES: &str = "failures:";

/// The words that open the block which holds what a failed example wrote
const OUTPUT_OPENS: &str = "---- ";

/// The words that close the opening line of such a block
const OUTPUT_CLOSES: &str = " stdout ----";

/// The word that opens the line of one example
const EXAMPLE_OPENS: &str = "test ";

/// The words that close the line of an example that failed
const EXAMPLE_FAILED: &str = " ... FAILED";

/// The words that open the summary of a run
const SUMMARY: &str = "test result: ";

/// The word that counts the examples which passed
const PASSED: &str = "passed";

/// The word that counts the examples which failed
const FAILED: &str = "failed";

/// What the test harness reported about the examples of one workspace
///
/// The report holds how many examples ran, the examples that failed with
/// what each of them wrote, and how many summaries the harness wrote. Cargo
/// tests one package at a time and the harness writes a summary per package,
/// so the count of the summaries says whether the harness reported at all,
/// which is what tells a run that examined nothing from a run that examined
/// something and found it good.
///
/// # Examples
///
/// ```
/// use rakko_test_rust_docs::DoctestReport;
///
/// let stdout = "running 1 test\ntest src/lib.rs - add (line 7) ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
///
/// let report = DoctestReport::read(stdout)?;
///
/// assert_eq!(report.ran(), 1);
/// # Ok::<(), rakko_test_rust_docs::ReadDoctestReportError>(())
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct DoctestReport {
    /// The examples that failed, in the order that the harness named them
    #[getset(get = "pub")]
    failures: Vec<DoctestFailure>,

    /// How many examples ran, which is those that passed and those that
    /// failed
    #[getset(get_copy = "pub")]
    ran: u64,

    /// How many summaries the harness wrote, which is one per package that
    /// cargo tested
    #[getset(get_copy = "pub")]
    summaries: usize,
}

impl DoctestReport {
    /// Reads the report of a run from what cargo wrote to its standard
    /// output
    ///
    /// The reading takes the examples that failed from the lines that name
    /// them, what each of them wrote from the block that follows, and the
    /// count from the summaries. It ignores every other line, so the records
    /// that cargo writes to the same stream do not disturb it, and neither
    /// does a line that a new version of the harness adds.
    ///
    /// What an example wrote is arbitrary text, and the reading takes it up
    /// to the next line that opens a block, closes the failures, or ends a
    /// package.
    ///
    /// # Errors
    ///
    /// Returns [`UnreadableSummary`][unreadable] when a summary does not
    /// count the examples that passed and the examples that failed. The
    /// shape belongs to a version of the harness, and a reading that
    /// skipped such a line would answer with a count that is too low.
    ///
    /// [unreadable]: ReadDoctestReportError::UnreadableSummary
    // testrustdocs[impl report.failures]
    // testrustdocs[impl report.ran]
    // testrustdocs[impl report.unreadable]
    pub fn read(stdout: &str) -> Result<Self, ReadDoctestReportError> {
        let mut names: Vec<&str> = Vec::new();
        let mut blocks: Vec<(&str, String)> = Vec::new();
        let mut open: Option<usize> = None;
        let mut ran = 0;
        let mut summaries = 0;

        for line in stdout.lines() {
            if let Some(name) = opens_output(line) {
                open = Some(blocks.len());
                blocks.push((name, String::new()));
            } else if let Some(counts) = line.strip_prefix(SUMMARY) {
                ran +=
                    examples(counts).ok_or_else(|| ReadDoctestReportError::UnreadableSummary {
                        line: line.to_owned(),
                    })?;
                summaries += 1;
                open = None;
            } else if line.trim_end() == FAILURES {
                open = None;
            } else if let Some(index) = open {
                let output = &mut blocks[index].1;
                output.push_str(line);
                output.push('\n');
            } else if let Some(name) = failed(line) {
                names.push(name);
            }
        }

        Ok(Self {
            failures: failures(names, blocks),
            ran,
            summaries,
        })
    }
}

/// Returns the examples that failed, each with what it wrote
///
/// The harness names an example by the file that documents it and the line
/// that the example starts on, and two workspaces of a project can hold two
/// examples of that name. Each block therefore answers one name once, in the
/// order that the harness wrote them, and an example without a block keeps
/// its name and no output.
// testrustdocs[impl report.failures]
fn failures(names: Vec<&str>, mut blocks: Vec<(&str, String)>) -> Vec<DoctestFailure> {
    names
        .into_iter()
        .map(|name| {
            let output = blocks
                .iter()
                .position(|(held, _)| *held == name)
                .map(|index| blocks.remove(index).1)
                .unwrap_or_default();

            DoctestFailure::new(name, output)
        })
        .collect()
}

/// Returns how many examples a summary counts, when it counts them
///
/// A summary names the examples that passed, that failed, that the harness
/// left out, and that a filter removed, in that order and each behind its
/// count. The reading looks for the two words that it needs and takes the
/// word in front of each of them, so a word that a new version adds changes
/// nothing. An example that the harness left out ran as little as one that a
/// filter removed, so neither counts.
// testrustdocs[impl report.ran]
// testrustdocs[impl report.unreadable]
fn examples(counts: &str) -> Option<u64> {
    let mut ran = 0;
    let mut passed = false;
    let mut failed = false;
    let mut previous = "";

    for word in counts.split([' ', ';']).filter(|word| !word.is_empty()) {
        if word == PASSED || word == FAILED {
            ran += previous.parse::<u64>().ok()?;
            passed |= word == PASSED;
            failed |= word == FAILED;
        }

        previous = word;
    }

    (passed && failed).then_some(ran)
}

/// Returns the name of the example that a line reports as failed
// testrustdocs[impl report.failures]
fn failed(line: &str) -> Option<&str> {
    line.strip_prefix(EXAMPLE_OPENS)?
        .strip_suffix(EXAMPLE_FAILED)
}

/// Returns the name of the example whose output a line opens
// testrustdocs[impl report.failures]
fn opens_output(line: &str) -> Option<&str> {
    line.strip_prefix(OUTPUT_OPENS)?.strip_suffix(OUTPUT_CLOSES)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What cargo and the harness wrote about a workspace with one example
    /// that passed
    const PASSING: &str = "{\"reason\":\"build-finished\",\"success\":true}\n\nrunning 1 test\ntest src/lib.rs - add (line 7) ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";

    /// What the harness wrote about a workspace with one example that
    /// panicked and one that passed
    const FAILING: &str = "running 2 tests\ntest src/lib.rs - add (line 5) ... FAILED\ntest src/lib.rs - sub (line 16) ... ok\n\nfailures:\n\n---- src/lib.rs - add (line 5) stdout ----\nTest executable failed (exit status: 101).\n\nstderr:\n\nthread 'main' panicked at src/lib.rs:5:1:\nassertion failed: false\n\n\nfailures:\n    src/lib.rs - add (line 5)\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s\n";

    // testrustdocs[verify report.ran]
    #[test]
    fn read_a_report_of_two_packages_sums_their_examples() {
        let report = DoctestReport::read(&format!("{PASSING}{PASSING}"));

        assert_eq!(report.map(|report| report.ran()).ok(), Some(2));
    }

    // testrustdocs[verify report.ran]
    #[test]
    fn read_a_report_of_two_packages_counts_their_summaries() {
        let report = DoctestReport::read(&format!("{PASSING}{PASSING}"));

        assert_eq!(report.map(|report| report.summaries()).ok(), Some(2));
    }

    // testrustdocs[verify report.ran]
    #[test]
    fn read_a_report_beside_the_records_of_cargo_counts_the_examples() {
        let report = DoctestReport::read(PASSING);

        assert_eq!(report.map(|report| report.ran()).ok(), Some(1));
    }

    // testrustdocs[verify report.ran]
    #[test]
    fn read_a_report_of_a_failing_run_counts_the_example_that_failed() {
        let report = DoctestReport::read(FAILING);

        assert_eq!(report.map(|report| report.ran()).ok(), Some(2));
    }

    // testrustdocs[verify report.failures]
    #[test]
    fn read_a_report_of_a_failing_run_names_the_example() {
        let report = DoctestReport::read(FAILING);

        assert_eq!(
            report
                .ok()
                .map(|report| report.failures().to_vec())
                .as_deref(),
            Some(
                [DoctestFailure::new(
                    "src/lib.rs - add (line 5)",
                    "Test executable failed (exit status: 101).\n\nstderr:\n\nthread 'main' panicked at src/lib.rs:5:1:\nassertion failed: false\n\n\n",
                )]
                .as_slice()
            )
        );
    }

    // testrustdocs[verify report.failures]
    #[test]
    fn read_a_report_of_a_passing_run_names_no_example() {
        let report = DoctestReport::read(PASSING);

        assert_eq!(report.map(|report| report.failures().len()).ok(), Some(0));
    }

    // testrustdocs[verify report.failures]
    #[test]
    fn read_a_report_whose_example_wrote_nothing_holds_the_name() {
        let stdout = "running 1 test\ntest src/lib.rs - add (line 5) ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";

        let report = DoctestReport::read(stdout);

        assert_eq!(
            report
                .ok()
                .map(|report| report.failures().to_vec())
                .as_deref(),
            Some([DoctestFailure::new("src/lib.rs - add (line 5)", "")].as_slice())
        );
    }

    // testrustdocs[verify report.ran]
    #[test]
    fn read_a_stream_without_a_summary_counts_nothing() {
        let report = DoctestReport::read(r#"{"reason":"build-finished","success":false}"#);

        assert_eq!(report.map(|report| report.summaries()).ok(), Some(0));
    }

    // testrustdocs[verify report.unreadable]
    #[test]
    fn read_a_summary_that_counts_nothing_holds_the_line() {
        let line = "test result: ok. everything went fine";

        let report = DoctestReport::read(line);

        assert!(
            matches!(
                &report,
                Err(ReadDoctestReportError::UnreadableSummary { line: held })
                    if held == line
            ),
            "expected an unreadable summary, got {report:?}"
        );
    }
}
