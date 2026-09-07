//! What rustfmt reported about a run
//!
//! Rustfmt reports as text on both of its streams, and this module reads
//! that text. The shape of the text belongs to a version of rustfmt, and
//! this module is the one place that knows it.

/// One problem that rustfmt reported about a file
mod problem;

use std::collections::HashSet;
use std::path::PathBuf;

use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

pub use self::problem::{RustfmtProblem, RustfmtProblemDetail};

/// The start of a line that names a diagnostic of rustfmt
///
/// Rustfmt writes a diagnostic the way the compiler does: a line that starts
/// with the level, then a line that points at the source.
const DIAGNOSTIC: &str = "error";

/// The separator between the level of a diagnostic and its message
const LEVEL_SEPARATOR: &str = ": ";

/// The start of the line that points at the source of a diagnostic
const LOCATION_ARROW: &str = "-->";

/// The start of a line that warns about the configuration of rustfmt
///
/// Rustfmt warns about an option that it does not know, and about an option
/// that its channel does not support, and then formats without the option.
const WARNING: &str = "Warning:";

/// What rustfmt reported about one run
///
/// A run lists files on its standard output: in a check, the files that
/// rustfmt would rewrite, and in a rewrite, the files that it rewrote. It
/// writes its diagnostics and its warnings to its standard error stream. The
/// report keeps the lines that carry an answer and ignores everything else,
/// so a line that a new version adds does not break the reading.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct RustfmtReport {
    /// The files that rustfmt listed, once each, in the order of the output
    #[getset(get = "pub")]
    listed: Vec<PathBuf>,

    /// The files that rustfmt cannot parse, once each
    #[getset(get = "pub")]
    invalid: Vec<RustfmtProblem>,

    /// The first warning of rustfmt about its configuration, when it wrote
    /// one
    #[getset(get = "pub")]
    warning: Option<String>,

    /// What rustfmt wrote to its standard error stream
    #[getset(get = "pub")]
    stderr: String,

    /// Whether the run ended with success
    #[getset(get_copy = "pub")]
    succeeded: bool,
}

impl RustfmtReport {
    /// Returns the problems of a check
    ///
    /// Every file that the check listed is a file that rustfmt would
    /// rewrite, and every diagnostic is a file that rustfmt cannot parse.
    pub fn problems(&self) -> Vec<RustfmtProblem> {
        let mut problems: Vec<RustfmtProblem> = self
            .listed
            .iter()
            .map(|path| RustfmtProblem::new(path.clone(), RustfmtProblemDetail::Unformatted))
            .collect();
        problems.extend(self.invalid.iter().cloned());

        problems
    }

    /// Reads the report of a run from what rustfmt wrote
    // formatrust[impl check.invalid]
    // formatrust[impl check.unformatted]
    pub fn read(execution: &Execution) -> Self {
        read(
            &execution.stdout().to_string_lossy(),
            &execution.stderr().to_string_lossy(),
            execution.status().success(),
        )
    }
}

/// Returns the path, the line, and the column of a location
///
/// A location reads `path:line:column`, and a path can hold a colon of its
/// own, so the reading starts from the end.
fn coordinates(location: &str) -> Option<(PathBuf, u32, u32)> {
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let path = parts.next()?;

    (!path.is_empty()).then(|| (PathBuf::from(path), line, column))
}

/// Returns the diagnostic that a header line and the lines below it describe
///
/// The header carries the message, and one of the next two lines points at
/// the source with an arrow. A header without an arrow describes nothing
/// that the action can place, and it is ignored.
fn diagnostic(message: &str, following: &[&str]) -> Option<RustfmtProblem> {
    let arrow = following
        .iter()
        .take(2)
        .find_map(|line| line.trim_start().strip_prefix(LOCATION_ARROW))?;
    let (path, line, column) = coordinates(arrow.trim())?;

    Some(RustfmtProblem::new(
        path,
        RustfmtProblemDetail::Invalid {
            line,
            column,
            message: message.to_owned(),
        },
    ))
}

/// Returns the message of a line that starts a diagnostic
///
/// The line starts with the level, which can carry a code in brackets, and
/// the message follows a colon. Rustfmt writes only errors this way.
fn header(line: &str) -> Option<&str> {
    let (level, message) = line.split_once(LEVEL_SEPARATOR)?;

    level.starts_with(DIAGNOSTIC).then_some(message.trim())
}

/// Reads a report from the two streams of a run
// formatrust[impl check.configuration]
// formatrust[impl check.invalid]
// formatrust[impl check.unformatted]
fn read(stdout: &str, stderr: &str, succeeded: bool) -> RustfmtReport {
    let mut listed = Vec::new();
    let mut seen = HashSet::new();

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let path = PathBuf::from(line);

        if seen.insert(path.clone()) {
            listed.push(path);
        }
    }

    let lines: Vec<&str> = stderr.lines().collect();
    let mut invalid = Vec::new();
    let mut warning = None;

    for (index, line) in lines.iter().enumerate() {
        if line.starts_with(WARNING) {
            if warning.is_none() {
                warning = Some(line.trim().to_owned());
            }
        } else if let Some(message) = header(line)
            && let Some(problem) = diagnostic(message, &lines[index + 1..])
            && !invalid.contains(&problem)
        {
            invalid.push(problem);
        }
    }

    RustfmtReport {
        listed,
        invalid,
        warning,
        stderr: stderr.to_owned(),
        succeeded,
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What rustfmt writes about a file that it cannot parse
    const INVALID: &str = "error: this file contains an unclosed delimiter\n --> /home/otter/project/src/broken.rs:1:18\n  |\n1 | pub fn broken( {\n  |              - -^\n\nError writing files: failed to resolve mod `broken`: cannot parse /home/otter/project/src/broken.rs\n";

    /// What rustfmt writes on its standard output about files it would
    /// rewrite, one of them twice
    const LISTED: &str = "/home/otter/project/src/lib.rs\n/home/otter/project/src/main.rs\n/home/otter/project/src/lib.rs\n";

    /// What rustfmt writes about an option that it does not know
    const UNKNOWN_OPTION: &str = "Warning: Unknown configuration option `no_such_option`\nWarning: Unknown configuration option `no_such_option`\n";

    // formatrust[verify check.configuration]
    #[test]
    fn read_a_warning_keeps_the_first_one() {
        let report = read("", UNKNOWN_OPTION, false);

        assert_eq!(
            report.warning().as_deref(),
            Some("Warning: Unknown configuration option `no_such_option`")
        );
    }

    // formatrust[verify check.invalid]
    #[test]
    fn read_a_diagnostic_reads_the_message() {
        let report = read("", INVALID, false);

        assert_eq!(
            report.invalid().first().map(RustfmtProblem::detail),
            Some(&RustfmtProblemDetail::Invalid {
                line: 1,
                column: 18,
                message: "this file contains an unclosed delimiter".to_owned(),
            })
        );
    }

    // formatrust[verify check.invalid]
    #[test]
    fn read_a_diagnostic_reads_the_path() {
        let report = read("", INVALID, false);

        assert_eq!(
            report.invalid().first().map(RustfmtProblem::path),
            Some(&PathBuf::from("/home/otter/project/src/broken.rs"))
        );
    }

    // formatrust[verify check.unformatted]
    #[test]
    fn read_a_file_listed_twice_keeps_one() {
        let report = read(LISTED, "", false);

        assert_eq!(
            report.listed(),
            &[
                PathBuf::from("/home/otter/project/src/lib.rs"),
                PathBuf::from("/home/otter/project/src/main.rs"),
            ]
        );
    }

    #[test]
    fn read_an_empty_report_finds_nothing() {
        let report = read("", "", true);

        assert_eq!(
            report,
            RustfmtReport {
                listed: Vec::new(),
                invalid: Vec::new(),
                warning: None,
                stderr: String::new(),
                succeeded: true,
            }
        );
    }

    // formatrust[verify check.unformatted]
    #[test]
    fn problems_of_a_check_name_the_listed_files() {
        let report = read(LISTED, "", false);

        let problems = report.problems();

        assert_eq!(
            problems.first(),
            Some(&RustfmtProblem::new(
                PathBuf::from("/home/otter/project/src/lib.rs"),
                RustfmtProblemDetail::Unformatted,
            ))
        );
    }

    #[test]
    fn problems_of_a_check_hold_the_invalid_files_after_the_listed_ones() {
        let report = read(LISTED, INVALID, false);

        let problems = report.problems();

        assert_eq!(problems.len(), 3);
    }
}
