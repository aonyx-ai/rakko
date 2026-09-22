//! The reading of the report that the test runner wrote
//!
//! The runner reports a run as TAP: one line per test that says whether it
//! passed, and an indented block below it that carries the place of the test
//! and, for a test that failed, what went wrong. The report closes with one
//! line per count of the run.
//!
//! This module turns that text into a [`Report`]. It judges nothing: the
//! caller decides what a failure means for the outcome of a run.
//!
//! The reading keeps only what a caller needs, which is the tests that failed
//! and the counts. It is strict about the shape all the same, because a line
//! that it cannot place belongs to a report that it does not know, and the
//! failures it collected from such a report are the ones that it happened to
//! understand.

/// The reading of one indented block of a report
pub mod block;
/// The error that leaves a caller without the report of a run
mod error;

use bon::Builder;
use getset::{CopyGetters, Getters};

pub use self::block::{Block, Scalar};
pub use self::error::ReadReportError;
use crate::failure::{Failure, Origin};

/// The text that opens every report of the runner
const HEADER: &str = "TAP version ";

/// The text that opens a line about a test that failed
const NOT_OK: &str = "not ok ";

/// The text that opens a line about a test that passed
const OK: &str = "ok ";

/// The text that separates the number of a test from its name
const NAME_OPEN: &str = " - ";

/// The character that opens a comment, and a directive behind the name of a
/// test
const HASH: char = '#';

/// The character that escapes a character of a name
const ESCAPE: char = '\\';

/// The text that opens the indented block below a line about a test
const BLOCK_OPEN: &str = "---";

/// The text that separates the two numbers of a plan
const PLAN: &str = "..";

/// The key of the block that names where the project declares a test
const LOCATION: &str = "location";

/// The key of the block that names why a test failed
const REASON: &str = "error";

/// The key of the block that names what kind of failure a test had
const FAILURE_TYPE: &str = "failureType";

/// The kind of failure of a test that only the tests below it failed
///
/// The runner reports a test that holds tests as failed when one of those
/// fails, and it reports the one below as well. The one below is the failure
/// that a reader repairs, so a caller wants one finding and not two.
const SUBTESTS_FAILED: &str = "subtestsFailed";

/// The name of the count that says how many tests the run ran
const TESTS: &str = "tests";

/// The text that joins two lines of a reason into one
///
/// A reason that the runner wrote over several lines carries the values that
/// disagreed on lines of their own. A finding message holds no formatting, so
/// the lines join with the separator that a reader expects between two parts
/// of a sentence.
const JOIN: char = ' ';

/// What the test runner reported about a run
///
/// The report carries the tests that failed and the count of the tests that
/// ran. A run that ran no test counted none, which is how a caller tells a
/// project that holds no test from one whose tests all passed: both report no
/// failure, and only the first reports no test.
///
/// A test that passed carries nothing that a caller needs, so the report keeps
/// no record of it beyond the count.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct Report {
    /// The tests that failed, in the order that the runner reported them
    #[getset(get = "pub")]
    failures: Vec<Failure>,

    /// How many tests the run ran
    #[getset(get_copy = "pub")]
    tests: u64,
}

/// Returns what the test runner reported about a run
///
/// The reading collects the tests that failed and the counts that close the
/// report. A test that the project marked as skipped or as still to do carries
/// a directive behind its name, and the runner counts neither as a failure
/// however it ended, so the reading leaves it out as well.
///
/// # Errors
///
/// Returns [`MissingHeader`][header] for text that opens no report,
/// [`MissingCounts`][counts] for a report that ends without the counts of the
/// run, and [`UnreadableLine`][unreadable] for a line that the reading cannot
/// place.
///
/// [counts]: ReadReportError::MissingCounts
/// [header]: ReadReportError::MissingHeader
/// [unreadable]: ReadReportError::UnreadableLine
// testtypescript[impl report.unreadable]
pub fn read(report: &str) -> Result<Report, ReadReportError> {
    let lines: Vec<&str> = report.lines().collect();

    let start = opening(&lines)?;
    let (body, counts) = split(&lines[start..])?;

    let failures = results(body)?;
    let tests = count(counts, TESTS).ok_or(ReadReportError::MissingCounts)?;

    Ok(Report::builder().failures(failures).tests(tests).build())
}

/// Returns the value of the named count, when the lines hold it
fn count(counts: &[&str], name: &str) -> Option<u64> {
    counts
        .iter()
        .filter_map(|line| stated(line))
        .find(|(key, _)| *key == name)
        .and_then(|(_, value)| value.parse().ok())
}

/// Returns the index of the line that opens the report
///
/// # Errors
///
/// Returns [`MissingHeader`][header] when no line opens a report.
///
/// [header]: ReadReportError::MissingHeader
fn opening(lines: &[&str]) -> Result<usize, ReadReportError> {
    let opening = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .filter(|index| lines[*index].starts_with(HEADER));

    opening.map_or(Err(ReadReportError::MissingHeader), |index| Ok(index + 1))
}

/// Returns the failures that the lines of a report state
///
/// A line about a test opens a result, and the block below it carries the
/// place and the reason. A result stays open until the next one starts or the
/// lines end, because only then is its block complete.
///
/// # Errors
///
/// Returns [`UnreadableLine`][unreadable] for a line that states no result and
/// is no part of the block of one.
///
/// [unreadable]: ReadReportError::UnreadableLine
fn results(lines: &[&str]) -> Result<Vec<Failure>, ReadReportError> {
    let mut failures = Vec::new();
    let mut open: Option<Pending> = None;
    let mut block: Option<Block> = None;

    for line in lines {
        if let Some(reading) = block.as_mut()
            && reading.open()
        {
            reading.read(line)?;
            continue;
        }

        if let Some(reading) = block.take()
            && let Some(pending) = open.as_mut()
        {
            pending.describe(&reading);
        }

        let trimmed = line.trim_start();

        if trimmed.is_empty() || trimmed.starts_with(HASH) || is_plan(trimmed) {
            continue;
        }

        if trimmed == BLOCK_OPEN {
            block = Some(Block::new(indent(line)));
            continue;
        }

        let Some(started) = start(trimmed) else {
            return Err(unreadable(line));
        };

        if let Some(previous) = open.replace(started) {
            previous.collect(&mut failures);
        }
    }

    if let Some(reading) = block
        && let Some(pending) = open.as_mut()
    {
        pending.describe(&reading);
    }

    if let Some(pending) = open {
        pending.collect(&mut failures);
    }

    Ok(failures)
}

/// Returns whether the line states a plan, which says how many tests follow
fn is_plan(line: &str) -> bool {
    line.split_once(PLAN)
        .is_some_and(|(first, second)| number(first) && number(second))
}

/// Returns the indentation of a line, in characters
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Returns whether the text is a number and nothing else
fn number(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|character| character.is_ascii_digit())
}

/// Returns the name of a test, with the escapes of the report undone
///
/// The report escapes the character that opens a comment and the character
/// that escapes, so that the name of a test always fits on one line and never
/// reads as a directive.
fn unescape(name: &str) -> String {
    let mut unescaped = String::with_capacity(name.len());
    let mut characters = name.chars();

    while let Some(character) = characters.next() {
        if character != ESCAPE {
            unescaped.push(character);
            continue;
        }

        match characters.next() {
            Some(escaped) => unescaped.push(escaped),
            None => unescaped.push(ESCAPE),
        }
    }

    unescaped
}

/// Returns the key and the value that a line of counts states
///
/// The report writes a count as a comment that holds the name and the number,
/// so the reading drops the character that opens the comment first.
fn stated(line: &str) -> Option<(&str, &str)> {
    line.trim().strip_prefix(HASH)?.trim().split_once(JOIN)
}

/// Returns the lines of a report, split into its results and its counts
///
/// The report closes with one line per count of the run, and each of them is a
/// comment that holds a name and a number. A test writes to its own output as
/// a comment as well, so the counts are the block of them that the report ends
/// with, and nothing before it.
///
/// # Errors
///
/// Returns [`MissingCounts`][counts] when the report ends with no such block.
///
/// [counts]: ReadReportError::MissingCounts
fn split<'a>(lines: &'a [&'a str]) -> Result<(&'a [&'a str], &'a [&'a str]), ReadReportError> {
    let mut opening = lines.len();

    while opening > 0 {
        let line = lines[opening - 1];

        if line.trim().is_empty() {
            opening -= 1;
            continue;
        }

        let Some((_, value)) = stated(line) else {
            break;
        };

        if line.starts_with(char::is_whitespace) || value.trim().is_empty() {
            break;
        }

        opening -= 1;
    }

    if opening == lines.len() {
        return Err(ReadReportError::MissingCounts);
    }

    Ok((&lines[..opening], &lines[opening..]))
}

/// Returns the result that the given line starts
///
/// Returns `None` when the line states no result at all.
fn start(line: &str) -> Option<Pending> {
    let failed = match line.strip_prefix(NOT_OK) {
        Some(_) => true,
        None => {
            line.strip_prefix(OK)?;
            false
        }
    };

    let (_, named) = line.split_once(NAME_OPEN)?;
    let (name, directive) = split_directive(named);

    Some(Pending {
        name: unescape(name),
        // A directive excuses whatever the test did, and the runner counts
        // such a test as neither passed nor failed.
        // testtypescript[impl result.excused]
        failed: failed && directive.is_none(),
        origin: None,
        reason: None,
        kind: None,
    })
}

/// Returns the name of a test and the directive that follows it
///
/// A directive says that the project excused the test, and it sits behind the
/// name after the character that opens a comment. That character is escaped
/// inside a name, so the first one that is not escaped opens the directive.
fn split_directive(named: &str) -> (&str, Option<&str>) {
    let mut escaped = false;

    for (index, character) in named.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match character {
            ESCAPE => escaped = true,
            HASH => return (named[..index].trim_end(), Some(named[index..].trim())),
            _ => {}
        }
    }

    (named, None)
}

/// Returns the error of a line that is no part of a report that this crate
/// reads
// testtypescript[impl report.unreadable]
fn unreadable(line: &str) -> ReadReportError {
    ReadReportError::UnreadableLine {
        line: line.trim().to_owned(),
    }
}

/// One result of a report whose block the reading has yet to see
///
/// The report writes the block of a result below the line that states it, so a
/// result is complete only once the next one starts or the report ends. This
/// holds the parts until then.
struct Pending {
    /// The name of the test, with the escapes of the report undone
    name: String,

    /// Whether the test counts as failed
    failed: bool,

    /// The place where the project declares the test
    origin: Option<Origin>,

    /// What the runner said about the failure
    reason: Option<String>,

    /// What kind of failure the runner gave the test
    kind: Option<String>,
}

impl Pending {
    /// Adds the failures of this result to the given list
    ///
    /// A result that passed adds nothing. Neither does one that failed only
    /// because a test below it failed, because that test is in the list
    /// already, at the place where a reader repairs it.
    // testtypescript[impl result.subtests]
    fn collect(self, failures: &mut Vec<Failure>) {
        if !self.failed || self.kind.as_deref() == Some(SUBTESTS_FAILED) {
            return;
        }

        failures.push(
            Failure::builder()
                .name(self.name)
                .maybe_origin(self.origin)
                .maybe_reason(self.reason)
                .build(),
        );
    }

    /// Takes the place, the reason, and the kind out of the block of a result
    // testtypescript[impl result.message]
    // testtypescript[impl result.position]
    // testtypescript[impl result.project]
    fn describe(&mut self, block: &Block) {
        self.origin = block.value(LOCATION).and_then(Scalar::place);
        self.reason = block.value(REASON).map(Scalar::flattened);
        self.kind = block.value(FAILURE_TYPE).map(Scalar::flattened);
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]
    #![allow(clippy::unwrap_used)]

    use rakko_test_utils::path;

    use super::*;

    /// A report of a run whose two tests both passed
    const PASSED: &str = "TAP version 13\n# Subtest: adds\nok 1 - adds\n  ---\n  duration_ms: \
                          0.6\n  type: 'test'\n  ...\n# Subtest: subtracts\nok 2 - subtracts\n  \
                          ---\n  duration_ms: 0.2\n  type: 'test'\n  ...\n1..2\n# tests 2\n# \
                          suites 0\n# pass 2\n# fail 0\n# cancelled 0\n# skipped 0\n# todo 0\n# \
                          duration_ms 90.9\n";

    /// A report of a run that ran no test at all
    const UNTESTED: &str = "TAP version 13\n1..0\n# tests 0\n# suites 0\n# pass 0\n# fail 0\n# \
                            cancelled 0\n# skipped 0\n# todo 0\n# duration_ms 7.7\n";

    /// A report of a run whose one test failed an assertion
    const FAILED: &str = "TAP version 13\n# Subtest: counts\nnot ok 1 - counts\n  ---\n  \
                          duration_ms: 0.6\n  type: 'test'\n  location: \
                          '/home/otter/project/src/add.test.ts:5:1'\n  failureType: \
                          'testCodeFailure'\n  error: |-\n    Expected values to be strictly \
                          equal:\n\n    2 !== 3\n\n  code: 'ERR_ASSERTION'\n  ...\n1..1\n# tests \
                          1\n# suites 0\n# pass 0\n# fail 1\n# cancelled 0\n# skipped 0\n# todo \
                          0\n# duration_ms 90.9\n";

    /// A report of a run whose suite holds the test that failed
    const NESTED: &str = "TAP version 13\n# Subtest: arithmetic\n    # Subtest: counts\n    not \
                          ok 1 - counts\n      ---\n      location: \
                          '/home/otter/project/src/add.test.ts:5:3'\n      failureType: \
                          'testCodeFailure'\n      error: 'it broke'\n      ...\n    1..1\nnot ok \
                          1 - arithmetic\n  ---\n  type: 'suite'\n  location: \
                          '/home/otter/project/src/add.test.ts:4:1'\n  failureType: \
                          'subtestsFailed'\n  error: '1 subtest failed'\n  ...\n1..1\n# tests 1\n# \
                          suites 1\n# pass 0\n# fail 1\n# cancelled 0\n# skipped 0\n# todo 0\n# \
                          duration_ms 90.9\n";

    /// Returns the report of a run whose one test carries the given lines
    fn one(result: &str, block: &str) -> String {
        format!(
            "TAP version 13\n{result}\n  ---\n{block}  ...\n1..1\n# tests 1\n# suites 0\n# pass \
             0\n# fail 1\n# cancelled 0\n# skipped 0\n# todo 0\n# duration_ms 9.9\n"
        )
    }

    #[test]
    fn read_of_a_report_of_a_run_that_passed_reports_no_failure() {
        let report = read(PASSED).unwrap();

        assert!(
            report.failures().is_empty(),
            "expected no failure, got {:?}",
            report.failures()
        );
    }

    // testtypescript[verify result.passed]
    #[test]
    fn read_of_a_report_of_a_run_that_passed_counts_its_tests() {
        let report = read(PASSED).unwrap();

        assert_eq!(report.tests(), 2);
    }

    // testtypescript[verify skip.untested]
    #[test]
    fn read_of_a_report_of_a_run_without_a_test_counts_none() {
        let report = read(UNTESTED).unwrap();

        assert_eq!(report.tests(), 0);
    }

    // testtypescript[verify result.failed]
    #[test]
    fn read_of_a_report_of_a_test_that_failed_names_the_test() {
        let report = read(FAILED).unwrap();

        assert_eq!(report.failures()[0].name(), "counts");
    }

    // testtypescript[verify result.message]
    #[test]
    fn read_of_a_report_of_a_test_that_failed_carries_the_reason_on_one_line() {
        let report = read(FAILED).unwrap();

        assert_eq!(
            report.failures()[0].reason().as_deref(),
            Some("Expected values to be strictly equal: 2 !== 3")
        );
    }

    // testtypescript[verify result.position]
    #[test]
    fn read_of_a_report_of_a_test_that_failed_carries_its_place() {
        let report = read(FAILED).unwrap();

        assert_eq!(
            report.failures()[0].origin(),
            &Some(
                Origin::builder()
                    .path(path("/home/otter/project/src/add.test.ts"))
                    .line(5)
                    .column(1)
                    .build()
            )
        );
    }

    // testtypescript[verify result.project]
    #[test]
    fn read_of_a_report_of_a_test_without_a_place_carries_none() {
        let report = read(&one("not ok 1 - counts", "  error: 'test failed'\n")).unwrap();

        assert_eq!(report.failures()[0].origin(), &None);
    }

    // testtypescript[verify result.subtests]
    #[test]
    fn read_of_a_report_of_a_suite_reports_only_the_test_below_it() {
        let report = read(NESTED).unwrap();

        assert_eq!(
            report
                .failures()
                .iter()
                .map(Failure::name)
                .collect::<Vec<_>>(),
            ["counts"]
        );
    }

    // testtypescript[verify result.excused]
    #[test]
    fn read_of_a_report_of_a_test_that_is_still_to_do_reports_no_failure() {
        let report = read(&one(
            "not ok 1 - counts # TODO",
            "  failureType: 'testCodeFailure'\n",
        ))
        .unwrap();

        assert!(
            report.failures().is_empty(),
            "expected no failure, got {:?}",
            report.failures()
        );
    }

    // The report escapes the character that opens a directive inside a name,
    // so a name that holds one is no directive and the test still counts.
    // testtypescript[verify result.excused]
    #[test]
    fn read_of_a_report_of_a_test_whose_name_holds_a_hash_reports_the_failure() {
        let report = read(&one(
            "not ok 1 - counts \\# twice",
            "  failureType: 'testCodeFailure'\n",
        ))
        .unwrap();

        assert_eq!(report.failures()[0].name(), "counts # twice");
    }

    // A test writes its own output into the report as a comment, and a line of
    // it can read exactly like a result. The reading takes it for a comment,
    // because that is what the runner wrote.
    #[test]
    fn read_of_a_report_whose_test_wrote_a_result_reports_no_failure() {
        let report =
            read(&PASSED.replace("# Subtest: adds", "# not ok 99 - forged\n# Subtest: adds"))
                .unwrap();

        assert!(
            report.failures().is_empty(),
            "expected no failure, got {:?}",
            report.failures()
        );
    }

    // testtypescript[verify report.unreadable]
    #[test]
    fn read_of_text_that_opens_no_report_stops() {
        let error = read("node: bad option: --test-reporter=tap\n").unwrap_err();

        assert_eq!(error, ReadReportError::MissingHeader);
    }

    // testtypescript[verify report.unreadable]
    #[test]
    fn read_of_a_report_without_the_counts_of_the_run_stops() {
        let error = read("TAP version 13\n# Subtest: adds\nok 1 - adds\n1..1\n").unwrap_err();

        assert_eq!(error, ReadReportError::MissingCounts);
    }

    // testtypescript[verify report.unreadable]
    #[test]
    fn read_of_a_report_with_a_line_that_states_no_result_stops() {
        let report = PASSED.replace("ok 1 - adds", "PASS adds");

        let error = read(&report).unwrap_err();

        assert!(
            matches!(&error, ReadReportError::UnreadableLine { line } if line == "PASS adds"),
            "expected the line that the reading could not place, got {error:?}"
        );
    }
}
