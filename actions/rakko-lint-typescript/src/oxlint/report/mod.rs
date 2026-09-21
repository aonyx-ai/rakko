//! The reading of the report that oxlint wrote
//!
//! Oxlint answers a run that asked for data with one JSON object, and this
//! module turns that object into an observation. The reading is where the
//! shape of a version of oxlint is known, so a report that does not fit stops
//! the caller instead of reaching it as an answer about the project.
//!
//! The object is not always the whole of what oxlint wrote. A run that found
//! no file to lint writes a sentence for a reader first and the object after
//! it, so the reading looks for the object rather than taking the output for
//! one.

/// The error that leaves a caller without the report of a run
mod error;

use std::path::PathBuf;

use serde::Deserialize;

pub use self::error::ReadReportError;
use crate::observation::Observation;
use crate::problem::{OxlintProblem, Severity};

/// The character that opens the report of oxlint
const REPORT_OPEN: char = '{';

/// The report that oxlint wrote about one run
///
/// Oxlint writes more than these fields, and the reading ignores the rest, so
/// a field that a new version adds does not break it.
#[derive(Deserialize)]
struct Report {
    /// The rules that the files of the run broke
    diagnostics: Vec<Diagnostic>,

    /// How many files the run examined
    number_of_files: usize,
}

/// One rule that a file of the run broke
#[derive(Deserialize)]
struct Diagnostic {
    /// What oxlint said about the rule and the file
    message: String,

    /// The rule that the file broke, as oxlint names it
    code: String,

    /// How the project weighs the rule
    severity: Severity,

    /// What oxlint suggests about the diagnostic
    ///
    /// Oxlint writes a suggestion for most of its rules and leaves the field
    /// out for the rest, so a reading that required one would stop on a rule
    /// that has nothing to suggest.
    #[serde(default)]
    help: Option<String>,

    /// The file that broke the rule, as oxlint wrote it
    filename: PathBuf,

    /// The places of the file that the diagnostic marks
    labels: Vec<Label>,
}

impl Diagnostic {
    /// Returns the problem that this diagnostic reports
    ///
    /// Oxlint writes one diagnostic for a rule however many places of the file
    /// it marks, and the problem sits at the first of them, which is the place
    /// that oxlint itself points a reader at.
    ///
    /// Returns `None` for a diagnostic that marks no place. Every diagnostic of
    /// oxlint marks at least one, so such a report belongs to a version that
    /// this crate does not know, and a problem without a place could only be
    /// reported somewhere it does not belong.
    // linttypescript[impl check.diagnostic]
    fn into_problem(self) -> Option<OxlintProblem> {
        let first = self.labels.first()?;

        Some(
            OxlintProblem::builder()
                .path(self.filename)
                .line(first.span.line)
                .column(first.span.column)
                .severity(self.severity)
                .code(self.code)
                .description(self.message)
                .maybe_help(self.help)
                .build(),
        )
    }
}

/// One place of a file that a diagnostic marks
#[derive(Deserialize)]
struct Label {
    /// Where in the file the place is
    span: Span,
}

/// Where in a file a place is
#[derive(Deserialize)]
struct Span {
    /// The line that the place starts on, starting at 1
    line: u32,

    /// The column that the place starts at, starting at 1
    column: u32,
}

/// Returns what one run of oxlint reported
///
/// The output of a run carries the report, and a run that found no file to lint
/// writes a sentence in front of it, so the reading starts at the report and
/// not at the output.
///
/// # Errors
///
/// Returns [`Absent`][absent] when the output holds no report, which is what a
/// run that ended before it linted anything leaves behind, and
/// [`Unreadable`][unreadable] when the report does not have the shape that this
/// crate reads.
///
/// [absent]: ReadReportError::Absent
/// [unreadable]: ReadReportError::Unreadable
// linttypescript[impl check.diagnostic]
// linttypescript[impl check.passed]
// linttypescript[impl check.severity]
// linttypescript[impl skip.unexamined]
pub fn read(output: &str) -> Result<Observation, ReadReportError> {
    // linttypescript[impl check.unreported]
    let start = output.find(REPORT_OPEN).ok_or(ReadReportError::Absent)?;

    let report: Report = serde_json::from_str(output[start..].trim())
        // linttypescript[impl check.unreadable]
        .map_err(|source| ReadReportError::Unreadable { source })?;

    let problems = report
        .diagnostics
        .into_iter()
        .map(Diagnostic::into_problem)
        // linttypescript[impl check.unreadable]
        .collect::<Option<Vec<_>>>()
        .ok_or(ReadReportError::Unreadable {
            source: serde::de::Error::missing_field("labels"),
        })?;

    Ok(Observation::builder()
        .problems(problems)
        .examined(report.number_of_files)
        .build())
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a report
    // which oxlint could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;
    use crate::problem::{OxlintProblem, Severity};

    /// The report of a run that examined one file and found two rules broken
    ///
    /// The text is what oxlint writes, down to the way it lays out the object,
    /// so that a change in the layout reaches this test.
    const BROKEN: &str = r#"{ "diagnostics": [{"message": "Variable 'unused' is declared but never used. Unused variables should start with a '_'.","code": "eslint(no-unused-vars)","severity": "warning","url": "https://oxc.rs/docs/guide/usage/linter/rules/eslint/no-unused-vars.html","help": "Consider removing this declaration.","filename": "index.ts","labels": [{"label": "'unused' is declared here","span": {"offset": 50,"length": 6,"line": 3,"column": 9}}]},
{"message": "`debugger` statement is not allowed","code": "eslint(no-debugger)","severity": "error","url": "https://oxc.rs/docs/guide/usage/linter/rules/eslint/no-debugger.html","help": "Remove the debugger statement","filename": "index.ts","labels": [{"span": {"offset": 32,"length": 9,"line": 2,"column": 3}}]}],
              "number_of_files": 1,
              "number_of_rules": 96,
              "threads_count": 10,
              "start_time": 0.005394834
            }
            "#;

    /// The report of a run that examined two files and found nothing
    const CLEAN: &str = r#"{ "diagnostics": [],
              "number_of_files": 2,
              "number_of_rules": 96,
              "threads_count": 10,
              "start_time": 0.00552075
            }
            "#;

    /// A diagnostic that oxlint wrote no suggestion for
    const CURT: &str = r#"{ "diagnostics": [{"message": "Identifier name `x` is too short","code": "eslint(id-length)","severity": "warning","filename": "index.ts","labels": [{"span": {"offset": 1,"length": 1,"line": 1,"column": 7}}]}],
              "number_of_files": 1
            }
            "#;

    /// A diagnostic that marks two places of a file
    const PAIRED: &str = r#"{ "diagnostics": [{"message": "Duplicate key 'k'","code": "eslint(no-dupe-keys)","severity": "warning","help": "Consider removing the duplicated key","filename": "index.ts","labels": [{"span": {"offset": 10,"length": 1,"line": 5,"column": 15}},{"span": {"offset": 16,"length": 1,"line": 5,"column": 21}}]}],
              "number_of_files": 1
            }
            "#;

    /// What oxlint writes when it refuses the configuration of a project
    const REJECTION: &str = "Failed to parse oxlint configuration file.\n\n  \
                             x Failed to parse oxlint config /home/otter/project/.oxlintrc.json.\n  \
                             | key must be a string at line 1 column 3\n";

    /// A report that holds a field of a shape that oxlint does not write
    const STRANGE: &str = r#"{"diagnostics": "none"}"#;

    /// The report of a run that found no file to lint
    ///
    /// Oxlint writes a sentence for a reader in front of the object in this
    /// case, and the object follows it on the same stream.
    const UNEXAMINED: &str = r#"No files found to lint. Please check your paths and ignore patterns.
{ "diagnostics": [],
              "number_of_files": 0,
              "number_of_rules": 96,
              "threads_count": 10,
              "start_time": 0.008903083
            }
            "#;

    /// A diagnostic that marks no place of a file
    const UNPLACED: &str = r#"{ "diagnostics": [{"message": "Duplicate key 'k'","code": "eslint(no-dupe-keys)","severity": "warning","filename": "index.ts","labels": []}],
              "number_of_files": 1
            }
            "#;

    /// Returns the first diagnostic of a report that holds one
    fn problem(report: &str) -> OxlintProblem {
        let observation = read(report).expect("the test reads a report that oxlint could write");

        observation
            .problems()
            .first()
            .expect("the test reads a report that holds a diagnostic")
            .clone()
    }

    // linttypescript[verify check.passed]
    #[test]
    fn read_of_a_clean_report_counts_the_files_of_the_run() {
        let observation = read(CLEAN).expect("the test reads the report of a run that passed");

        assert_eq!(observation.examined(), 2);
    }

    // linttypescript[verify check.passed]
    #[test]
    fn read_of_a_clean_report_holds_no_problem() {
        let observation = read(CLEAN).expect("the test reads the report of a run that passed");

        assert!(
            observation.problems().is_empty(),
            "expected no problem, got {:?}",
            observation.problems()
        );
    }

    // linttypescript[verify check.severity]
    #[test]
    fn read_of_a_diagnostic_carries_the_severity_of_oxlint() {
        let problem = problem(BROKEN);

        assert_eq!(problem.severity(), Severity::Warning);
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_carries_the_words_of_oxlint() {
        let problem = problem(BROKEN);

        assert_eq!(
            problem.message(),
            "[warning] eslint(no-unused-vars): Variable 'unused' is declared but never \
             used. Unused variables should start with a '_'. \
             help: Consider removing this declaration."
        );
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_names_the_file() {
        let problem = problem(BROKEN);

        assert_eq!(problem.path(), &PathBuf::from("index.ts"));
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_sits_at_the_place_that_oxlint_marked() {
        let problem = problem(BROKEN);

        assert_eq!((problem.line(), problem.column()), (3, 9));
    }

    // linttypescript[verify check.unreadable]
    #[test]
    fn read_of_a_diagnostic_that_marks_no_place_stops_the_reading() {
        let observation = read(UNPLACED);

        assert!(
            observation.is_err(),
            "expected the reading to stop, got {observation:?}"
        );
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_that_marks_two_places_holds_one_problem() {
        let observation = read(PAIRED).expect("the test reads a report that oxlint could write");

        assert_eq!(observation.problems().len(), 1);
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_that_marks_two_places_sits_at_the_first() {
        let problem = problem(PAIRED);

        assert_eq!((problem.line(), problem.column()), (5, 15));
    }

    // linttypescript[verify check.diagnostic]
    #[test]
    fn read_of_a_diagnostic_without_a_suggestion_holds_none() {
        let problem = problem(CURT);

        assert_eq!(problem.help(), &None);
    }

    // linttypescript[verify check.unreadable]
    #[test]
    fn read_of_a_report_of_another_shape_stops_the_reading() {
        let error = read(STRANGE).expect_err("the test reads a report that oxlint could not write");

        assert!(
            matches!(error, ReadReportError::Unreadable { .. }),
            "expected an unreadable report, got {error:?}"
        );
    }

    // linttypescript[verify skip.unexamined]
    #[test]
    fn read_of_a_report_that_a_sentence_precedes_holds_the_report() {
        let observation =
            read(UNEXAMINED).expect("the test reads a report that oxlint could write");

        assert_eq!(observation.examined(), 0);
    }

    // linttypescript[verify check.unreported]
    #[test]
    fn read_of_an_absent_report_names_the_condition() {
        let error = read(REJECTION).expect_err("the test reads what oxlint wrote instead");

        assert!(
            matches!(error, ReadReportError::Absent),
            "expected an absent report, got {error:?}"
        );
    }
}
