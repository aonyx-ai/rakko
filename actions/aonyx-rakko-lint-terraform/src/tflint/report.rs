//! The reading of the report that TFLint wrote

use std::path::PathBuf;

use rakko::{Position, Span};
use serde::Deserialize;

use crate::issue::{Place, Severity, TflintIssue};

/// The report that TFLint writes in its JSON format
///
/// TFLint writes more than these fields, and the reading ignores the rest. So
/// a field that a new version adds does not break it.
#[derive(Deserialize)]
pub(super) struct Report {
    /// The problems that the rules found
    issues: Vec<Issue>,

    /// The errors that kept TFLint from inspecting a module
    errors: Vec<ReportedError>,
}

/// A problem that a rule of TFLint found
#[derive(Deserialize)]
struct Issue {
    /// The rule that found the problem
    rule: Rule,

    /// What the rule says about the problem
    message: String,

    /// Where the problem is
    range: Range,
}

/// A rule of TFLint
#[derive(Deserialize)]
struct Rule {
    /// The name of the rule
    name: String,

    /// How much the rule thinks that the problem matters
    severity: Severity,
}

/// The part of a file that an issue is about
#[derive(Deserialize)]
struct Range {
    /// The path of the file, relative to the directory that TFLint started in
    ///
    /// An issue about a module as a whole gives no file.
    filename: String,

    /// The first character of the range
    start: Point,

    /// The character after the range
    end: Point,
}

/// A position in a file, as TFLint reports it
#[derive(Deserialize)]
struct Point {
    /// The line, starting at 1
    line: u32,

    /// The column, starting at 1
    column: u32,
}

impl Point {
    /// Returns the position of a finding that this point gives
    ///
    /// TFLint and Rakko both count lines and columns from 1.
    fn position(&self) -> Position {
        Position::builder()
            .line(self.line)
            .column(self.column)
            .build()
    }
}

/// An error that kept TFLint from inspecting a module
#[derive(Deserialize)]
struct ReportedError {
    /// What TFLint says about the error
    message: String,
}

impl From<Issue> for TflintIssue {
    /// Turns one issue of the report into an issue of the crate
    fn from(issue: Issue) -> Self {
        let Issue {
            rule: Rule { name, severity },
            message,
            range:
                Range {
                    filename,
                    start,
                    end,
                },
        } = issue;

        let place = if filename.is_empty() {
            Place::Module
        } else {
            Place::Range {
                path: PathBuf::from(filename),
                span: Span::builder()
                    .start(start.position())
                    .end(end.position())
                    .build(),
            }
        };

        Self::builder()
            .place(place)
            .severity(severity)
            .rule(name)
            .description(message)
            .build()
    }
}

impl Report {
    /// Returns the issues of the report, in the order that TFLint wrote them
    pub(super) fn issues(self) -> Vec<TflintIssue> {
        self.issues.into_iter().map(TflintIssue::from).collect()
    }

    /// Returns the messages of the errors, without the sequences of color
    ///
    /// TFLint quotes the diagnostics of a module that it cannot load. The
    /// quote keeps the colors of the diagnostics, even when the run asks for
    /// none.
    pub(super) fn errors(&self) -> Vec<String> {
        self.errors
            .iter()
            .map(|error| plain(error.message.trim()))
            .collect()
    }
}

/// Returns the report that TFLint wrote
///
/// # Errors
///
/// Returns the error of a report that is not the object that TFLint writes. A
/// reading that tolerated such a report would let a run pass while the issues
/// of the run went unread.
pub(super) fn read(report: &[u8]) -> Result<Report, serde_json::Error> {
    serde_json::from_slice(report)
}

/// Returns the text of a message without the sequences that color it
///
/// A sequence of color starts with the escape character and a bracket, and it
/// ends with the first letter after them.
fn plain(message: &str) -> String {
    let mut text = String::with_capacity(message.len());
    let mut characters = message.chars().peekable();

    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            for inner in characters.by_ref() {
                if inner.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            text.push(character);
        }
    }

    text
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// A report that TFLint 0.66.0 wrote for a module with one issue
    const ONE_ISSUE: &str = r#"{"issues":[{"rule":{"name":"terraform_typed_variables","severity":"warning","link":"https://github.com/terraform-linters/tflint-ruleset-terraform/blob/v0.17.0/docs/rules/terraform_typed_variables.md"},"message":"`unused` variable has no type","range":{"filename":"mod/main.tf","start":{"line":1,"column":1},"end":{"line":1,"column":18}},"callers":[],"fixable":false,"fixed":false}],"errors":[]}"#;

    /// A report about a module without a file
    const MODULE_ISSUE: &str = r#"{"issues":[{"rule":{"name":"terraform_required_version","severity":"warning"},"message":"terraform \"required_version\" attribute is required","range":{"filename":"","start":{"line":0,"column":0},"end":{"line":0,"column":0}}}],"errors":[]}"#;

    /// A report of a module that TFLint could not load
    const ONE_ERROR: &str = r#"{"issues":[],"errors":[{"message":"Failed to load\n\n\u001b[31mError\u001b[0m: Unclosed configuration block\n","severity":"error"}]}"#;

    #[test]
    fn errors_of_a_report_lose_their_colors() {
        let report = read(ONE_ERROR.as_bytes()).unwrap();

        assert_eq!(
            report.errors(),
            ["Failed to load\n\nError: Unclosed configuration block"]
        );
    }

    #[test]
    fn issue_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Issue>();
    }

    #[test]
    fn issues_of_a_report_cover_the_range_of_tflint() {
        let issues = read(ONE_ISSUE.as_bytes()).unwrap().issues();

        assert_eq!(
            issues.first().map(TflintIssue::place),
            Some(&Place::Range {
                path: PathBuf::from("mod/main.tf"),
                span: Span::builder()
                    .start(Position::builder().line(1).column(1).build())
                    .end(Position::builder().line(1).column(18).build())
                    .build(),
            })
        );
    }

    #[test]
    fn issues_of_a_report_hold_the_message_of_tflint() {
        let issues = read(ONE_ISSUE.as_bytes()).unwrap().issues();

        assert_eq!(
            issues.first().map(TflintIssue::message),
            Some("[warning] `unused` variable has no type (terraform_typed_variables)".to_owned())
        );
    }

    #[test]
    fn issues_without_a_file_belong_to_the_module() {
        let issues = read(MODULE_ISSUE.as_bytes()).unwrap().issues();

        assert_eq!(issues.first().map(TflintIssue::place), Some(&Place::Module));
    }

    #[test]
    fn plain_text_stays_as_it_is() {
        let text = plain("Failed to load the root module");

        assert_eq!(text, "Failed to load the root module");
    }

    #[test]
    fn point_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Point>();
    }

    #[test]
    fn range_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Range>();
    }

    #[test]
    fn read_of_an_empty_report_fails() {
        let report = read(b"");

        assert!(report.is_err(), "expected the reading to fail");
    }

    #[test]
    fn read_of_an_unknown_severity_fails() {
        let report = read(ONE_ISSUE.replace("\"warning\"", "\"fatal\"").as_bytes());

        assert!(report.is_err(), "expected the reading to fail");
    }

    #[test]
    fn report_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Report>();
    }

    #[test]
    fn reported_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ReportedError>();
    }

    #[test]
    fn rule_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Rule>();
    }
}
