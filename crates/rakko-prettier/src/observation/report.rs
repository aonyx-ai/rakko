use std::path::PathBuf;

use super::Observation;
use crate::operation::Operation;
use crate::problem::{PrettierProblem, ProblemDetail};

/// The prefix of a line that reports something prettier could not do
const ERROR_PREFIX: &str = "[error] ";

/// The prefix of a line that reports something prettier decided to ignore
const WARNING_PREFIX: &str = "[warn] ";

/// The marker of the line that reports a file prettier could not open
const UNREADABLE: &str = "Unable to read file \"";

/// The marker of the line that reports a rejected configuration file
const INVALID_CONFIGURATION: &str = "Invalid configuration for file \"";

/// The marker of the line that reports a pattern without a match
const UNMATCHED_PATTERN: &str = "No files matching the pattern were found";

/// The marker of the line that reports an option prettier did not apply
///
/// Prettier writes the line and then runs without the option, so the line is
/// the only trace of a configuration that did not reach the run.
const IGNORED_OPTION: &str = "Ignored unknown option";

/// The suffix that marks a file a rewrite examined and left alone
const UNCHANGED: &str = " (unchanged)";

/// The separator between the path of a file and what a line says about it
const DETAIL_SEPARATOR: &str = ": ";

/// The separator between the line and the column of a position
const POSITION_SEPARATOR: char = ':';

/// The suffix of a duration in seconds, which also ends one in milliseconds
const SECONDS: &str = "s";

/// Reads what one run of prettier produced
///
/// The files that a run named arrive on the standard output stream, and what
/// prettier could not do arrives on the standard error stream. The reading
/// walks both, recognizes the lines that carry an answer, and ignores the
/// rest.
pub(super) fn read(
    stdout: &str,
    stderr: &str,
    succeeded: bool,
    operation: Operation,
) -> Observation {
    let mut observation = Observation {
        problems: Vec::new(),
        rejected_configuration: None,
        rewritten: Vec::new(),
        stderr: stderr.to_owned(),
        succeeded,
        unmatched_pattern: false,
    };

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        match operation {
            // prettier[impl report.unformatted]
            Operation::Report => observation.problems.push(PrettierProblem::new(
                PathBuf::from(line),
                ProblemDetail::Unformatted,
            )),
            // prettier[impl report.rewritten]
            Operation::Rewrite => {
                if let Some(path) = rewritten(line) {
                    observation.rewritten.push(PathBuf::from(path));
                }
            }
        }
    }

    let lines: Vec<&str> = stderr.lines().collect();

    for (index, line) in lines.iter().enumerate() {
        // prettier[impl report.configuration]
        if let Some(warning) = line.strip_prefix(WARNING_PREFIX) {
            if warning.starts_with(IGNORED_OPTION) && observation.rejected_configuration.is_none() {
                observation.rejected_configuration = Some(warning.trim().to_owned());
            }

            continue;
        }

        let Some(report) = line.strip_prefix(ERROR_PREFIX) else {
            continue;
        };

        // prettier[impl report.configuration]
        if report.starts_with(INVALID_CONFIGURATION) {
            if observation.rejected_configuration.is_none() {
                observation.rejected_configuration = Some(rejection(report, &lines[index + 1..]));
            }
        // prettier[impl report.pattern]
        } else if report.starts_with(UNMATCHED_PATTERN) {
            observation.unmatched_pattern = true;
        // prettier[impl report.unreadable]
        } else if let Some(path) = quoted_path(report, UNREADABLE) {
            let detail = ProblemDetail::Unreadable {
                reason: reason(&lines[index + 1..]),
            };

            observation
                .problems
                .push(PrettierProblem::new(path, detail));
        // prettier[impl report.diagnostic]
        } else if let Some(problem) = diagnostic(report) {
            observation.problems.push(problem);
        }
    }

    observation
}

/// Returns the problem that a line about a file prettier could not parse holds
///
/// Prettier writes the path, the message, and the position of the failure on
/// one line, and draws the offending text below it. The excerpt carries the
/// same prefix as the line above, so the position tells the two apart: only
/// the line that reports the failure ends with one.
fn diagnostic(report: &str) -> Option<PrettierProblem> {
    let (line, column) = position(report)?;
    let (path, message) = report.split_once(DETAIL_SEPARATOR)?;
    let message = message.rsplit_once('(')?.0.trim();

    let detail = ProblemDetail::Diagnostic {
        line,
        column,
        message: message.to_owned(),
    };

    Some(PrettierProblem::new(PathBuf::from(path), detail))
}

/// Returns whether a word of a line states how long prettier took
fn duration(word: &str) -> bool {
    word.ends_with(SECONDS) && word.starts_with(|character: char| character.is_ascii_digit())
}

/// Returns the position that closes a line, if the line carries one
///
/// Prettier ends the report of a failure with the line and the column in
/// parentheses, such as `(1:10)`, and both count from 1.
fn position(report: &str) -> Option<(u32, u32)> {
    let inside = report.strip_suffix(')')?.rsplit_once('(')?.1;
    let (line, column) = inside.split_once(POSITION_SEPARATOR)?;

    Some((line.trim().parse().ok()?, column.trim().parse().ok()?))
}

/// Returns the path that a marker of a line quotes, if the line carries it
fn quoted_path(report: &str, marker: &str) -> Option<PathBuf> {
    let quoted = report.strip_prefix(marker)?;

    Some(PathBuf::from(quoted.split_once('"')?.0))
}

/// Returns what prettier said about a configuration file that it rejected
///
/// Prettier states the file that it was about to format, and writes the
/// reason on the lines below. It repeats the block for every file, so the
/// reading stops at the next line that carries an answer of its own.
fn rejection(report: &str, rest: &[&str]) -> String {
    let mut details = vec![report.trim().to_owned()];

    for line in rest {
        let Some(detail) = line.strip_prefix(ERROR_PREFIX) else {
            break;
        };

        if detail.starts_with(INVALID_CONFIGURATION)
            || detail.starts_with(UNMATCHED_PATTERN)
            || detail.starts_with(UNREADABLE)
        {
            break;
        }

        details.push(detail.trim().to_owned());
    }

    details.join(" ")
}

/// Returns the reason that follows the report of a file prettier could not
/// read
///
/// Prettier names the file on one line and the reason of the operating system
/// on the next. A block without that line leaves the reason empty, and the
/// caller reports a file that prettier refused without saying why.
fn reason(rest: &[&str]) -> String {
    rest.first()
        .and_then(|line| line.strip_prefix(ERROR_PREFIX))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Returns the file that a line of a rewrite reports as changed
///
/// A rewrite names every file that it examined, states how long the file took,
/// and marks the files that it left alone. A path can hold a space, so the
/// reading takes the last word of the line and keeps it unless it states a
/// duration.
fn rewritten(line: &str) -> Option<&str> {
    if line.ends_with(UNCHANGED) {
        return None;
    }

    match line.rsplit_once(' ') {
        Some((path, last)) if duration(last) => Some(path),
        _ => Some(line),
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// The files that a report names as ones a rewrite would change
    const DIFFERENT: &str = "sub/messy.md\nREADME.md\n";

    /// The report of a run over a file that prettier cannot parse
    const INVALID: &str = "[error] deep/bad.json: SyntaxError: Unexpected token (1:10)\n[error] > 1 | { \"a\": 1,, }\n[error]     |          ^\n[error]   2 |\n";

    /// The report of a run whose configuration prettier could not read
    const REJECTED: &str = "[error] Invalid configuration for file \"/home/otter/project/a.md\":\n[error] JSON Error in /home/otter/project/.prettierrc.json:\n[error] Unexpected token \",\" is not valid JSON\n";

    /// The report of a run whose configuration named an option prettier does
    /// not know
    const IGNORED: &str = "[warn] Ignored unknown option { notAnOption: 5 }.\n[warn] Ignored unknown option { notAnOption: 5 }.\n";

    /// The report of a run whose pattern matched no file
    const UNMATCHED: &str = "[error] No files matching the pattern were found: \"**/*.css\".\n";

    /// The report of a run over a file that prettier cannot open
    const REFUSED: &str = "[error] Unable to read file \"deep/no.md\":\n[error] EACCES: permission denied, open '/home/otter/project/deep/no.md'\n";

    /// The files that a rewrite named, one of them left alone
    const WRITTEN: &str = "sub/messy.md 11ms\nREADME.md 1ms (unchanged)\n";

    /// Returns the detail of the only problem of a report
    fn detail(stderr: &str) -> ProblemDetail {
        let observation = read("", stderr, false, Operation::Report);

        let [problem] = observation.problems.as_slice() else {
            panic!("expected one problem, got {:?}", observation.problems);
        };

        problem.detail().clone()
    }

    #[test]
    fn read_a_clean_report_finds_no_problem() {
        let observation = read("", "", true, Operation::Report);

        assert!(observation.problems.is_empty());
    }

    // prettier[verify report.diagnostic]
    #[test]
    fn read_a_diagnostic_places_it_in_the_file() {
        let detail = detail(INVALID);

        assert_eq!(
            detail,
            ProblemDetail::Diagnostic {
                line: 1,
                column: 10,
                message: "SyntaxError: Unexpected token".to_owned(),
            }
        );
    }

    // prettier[verify report.status]
    #[test]
    fn read_a_failed_run_reports_the_failure() {
        let observation = read("", INVALID, false, Operation::Report);

        assert!(!observation.succeeded);
    }

    // prettier[verify report.pattern]
    #[test]
    fn read_a_pattern_without_a_match_reports_it() {
        let observation = read("", UNMATCHED, false, Operation::Report);

        assert!(observation.unmatched_pattern);
    }

    // prettier[verify report.configuration]
    #[test]
    fn read_a_rejected_configuration_reports_what_prettier_said() {
        let observation = read("", REJECTED, false, Operation::Report);

        assert_eq!(
            observation.rejected_configuration.as_deref(),
            Some(
                "Invalid configuration for file \"/home/otter/project/a.md\": JSON Error in /home/otter/project/.prettierrc.json: Unexpected token \",\" is not valid JSON"
            )
        );
    }

    // prettier[verify report.unformatted]
    #[test]
    fn read_a_report_names_the_files_that_differ() {
        let observation = read(DIFFERENT, "", false, Operation::Report);

        let paths: Vec<&PathBuf> = observation
            .problems
            .iter()
            .map(PrettierProblem::path)
            .collect();

        assert_eq!(
            paths,
            [&PathBuf::from("sub/messy.md"), &PathBuf::from("README.md")]
        );
    }

    // prettier[verify report.rewritten]
    #[test]
    fn read_a_rewrite_names_the_files_that_changed() {
        let observation = read(WRITTEN, "", true, Operation::Rewrite);

        assert_eq!(observation.rewritten, [PathBuf::from("sub/messy.md")]);
    }

    // prettier[verify report.rewritten]
    #[test]
    fn read_a_rewrite_reports_no_problem_of_its_own() {
        let observation = read(WRITTEN, "", true, Operation::Rewrite);

        assert!(observation.problems.is_empty());
    }

    // prettier[verify report.configuration]
    #[test]
    fn read_an_ignored_option_reports_what_prettier_said() {
        let observation = read("", IGNORED, true, Operation::Report);

        assert_eq!(
            observation.rejected_configuration.as_deref(),
            Some("Ignored unknown option { notAnOption: 5 }.")
        );
    }

    // prettier[verify report.unreadable]
    #[test]
    fn read_an_unreadable_file_holds_the_reason() {
        let detail = detail(REFUSED);

        assert_eq!(
            detail,
            ProblemDetail::Unreadable {
                reason: "EACCES: permission denied, open '/home/otter/project/deep/no.md'"
                    .to_owned(),
            }
        );
    }
}
