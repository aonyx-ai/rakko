//! The reading of the report that yamllint wrote
//!
//! Yamllint writes one line per problem in the parsable format, and this
//! module turns those lines into problems. The format carries no escaping, so
//! the reading takes a line apart at the marks that yamllint puts between the
//! fields.

/// The error that stops the reading of a report
mod error;

use std::path::PathBuf;

pub use self::error::ReadReportError;
use crate::problem::{ProblemLevel, YamllintProblem};

/// The text between the position of a problem and its level
const BEFORE_LEVEL: &str = ": [";

/// The text between the level of a problem and its description
const AFTER_LEVEL: &str = "] ";

/// The mark between the fields of the position of a problem
const POSITION: char = ':';

/// Returns the problems that a report of yamllint holds
///
/// A run that reported nothing writes nothing, and an empty report holds no
/// problem. Every other line is one problem.
///
/// # Errors
///
/// Returns [`UnreadableLine`][unreadable] for a line that does not report a
/// problem. A reading that skipped such a line would drop a problem of the
/// project without a word, and a reader would never learn that the report
/// held more than the findings that arrived.
///
/// [unreadable]: ReadReportError::UnreadableLine
pub fn problems(report: &str) -> Result<Vec<YamllintProblem>, ReadReportError> {
    report
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            problem(line).ok_or_else(|| ReadReportError::UnreadableLine {
                line: line.to_owned(),
            })
        })
        .collect()
}

/// Returns the problem that one line of a report describes
///
/// Yamllint writes the path, the line, the column, the level, and the
/// description of a problem, in that order and without escaping any of them.
/// The reading therefore takes the line apart from the marks that separate
/// the fields, and it takes the path from what is left. A path that holds a
/// mark of its own keeps it, because the position is read from the end of
/// that part and the description from the start of the rest.
///
/// Returns `None` for a line that does not carry all five fields.
fn problem(line: &str) -> Option<YamllintProblem> {
    let (place, rest) = line.split_once(BEFORE_LEVEL)?;
    let (level, description) = rest.split_once(AFTER_LEVEL)?;

    let (place, column) = place.rsplit_once(POSITION)?;
    let (path, number) = place.rsplit_once(POSITION)?;

    Some(YamllintProblem::new(
        PathBuf::from(path),
        number.parse().ok()?,
        column.parse().ok()?,
        ProblemLevel::parse(level)?,
        description.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which yamllint could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A problem of a rule that the project treats as an error
    const ERROR: &str = "./sub/a.yaml:3:5: [error] trailing spaces (trailing-spaces)";

    /// A problem of a rule that the project treats as a warning
    const WARNING: &str =
        "./notes.yaml:2:9: [warning] truthy value should be one of [false, true] (truthy)";

    /// What yamllint writes when a file is not YAML at all
    ///
    /// The description of the rule holds the marks that separate the fields
    /// of a line, so a reading that searched for them from the end would take
    /// the line apart in the wrong place.
    const SYNTAX: &str =
        "./a.yaml:3:1: [error] syntax error: expected ',' or ']', but got '<stream end>' (syntax)";

    /// A line of a report that this crate cannot read
    const UNREADABLE: &str = "Traceback (most recent call last):";

    /// Returns the single problem of a report that holds one
    fn problem(report: &str) -> YamllintProblem {
        let mut problems =
            problems(report).expect("the test reads a report that yamllint could write");

        problems
            .pop()
            .expect("the test reads a report that holds one problem")
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_problem_name_the_column_of_yamllint() {
        let problem = problem(ERROR);

        assert_eq!(problem.column(), 5);
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_problem_name_the_file() {
        let problem = problem(ERROR);

        assert_eq!(problem.path(), &PathBuf::from("./sub/a.yaml"));
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_problem_sit_on_the_line_of_yamllint() {
        let problem = problem(ERROR);

        assert_eq!(problem.line(), 3);
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_problem_carry_the_description_of_yamllint() {
        let problem = problem(ERROR);

        assert_eq!(problem.description(), "trailing spaces (trailing-spaces)");
    }

    // lintyaml[verify check.level]
    #[test]
    fn problems_of_a_warning_carry_the_warning_level() {
        let problem = problem(WARNING);

        assert_eq!(problem.level(), ProblemLevel::Warning);
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_syntax_error_keep_the_whole_description() {
        let problem = problem(SYNTAX);

        assert_eq!(
            problem.description(),
            "syntax error: expected ',' or ']', but got '<stream end>' (syntax)"
        );
    }

    // lintyaml[verify check.problem]
    #[test]
    fn problems_of_a_report_hold_one_problem_per_line() {
        let report = format!("{ERROR}\n{WARNING}\n");

        let problems = problems(&report).expect("the test reads a report of two problems");

        assert_eq!(problems.len(), 2);
    }

    // lintyaml[verify check.passed]
    #[test]
    fn problems_of_an_empty_report_hold_nothing() {
        let problems = problems("  \n").expect("the test reads the report of a run that passed");

        assert!(problems.is_empty(), "expected no problem, got {problems:?}");
    }

    // lintyaml[verify check.unreadable]
    #[test]
    fn problems_of_a_line_that_reports_nothing_stop_the_reading() {
        let problems = problems(UNREADABLE);

        assert!(
            problems.is_err(),
            "expected the reading to stop, got {problems:?}"
        );
    }

    // lintyaml[verify check.unreadable]
    #[test]
    fn problems_of_an_unknown_level_stop_the_reading() {
        let problems = problems("./a.yaml:1:1: [notice] a level of a later yamllint (rule)");

        assert!(
            problems.is_err(),
            "expected the reading to stop, got {problems:?}"
        );
    }
}
