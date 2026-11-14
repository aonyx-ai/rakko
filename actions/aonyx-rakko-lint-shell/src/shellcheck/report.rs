//! The reading of the report that ShellCheck wrote

use std::path::PathBuf;

use rakko::{Position, Span};
use serde::Deserialize;

use crate::comment::{CommentLevel, ShellcheckComment};

/// The report that ShellCheck writes in the `json1` format
///
/// ShellCheck writes one object with the comments of every script that the run
/// gave it. A run that found nothing writes the object with no comment.
#[derive(Deserialize)]
struct Report {
    /// The comments of the run, in the order of the scripts
    comments: Vec<Record>,
}

/// One comment of the report that ShellCheck wrote
///
/// ShellCheck writes more than these fields, such as a fix that it proposes.
/// The reading ignores the rest, so a field that a new version adds does not
/// break it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    /// The path of the script, as the command line gave it
    file: PathBuf,

    /// The line where the range of the comment starts, starting at 1
    line: u32,

    /// The column where the range of the comment starts, starting at 1
    column: u32,

    /// The line where the range of the comment ends, starting at 1
    end_line: u32,

    /// The first column after the range of the comment, starting at 1
    end_column: u32,

    /// How much ShellCheck thinks that the comment matters
    level: CommentLevel,

    /// The number of the check that wrote the comment
    code: u32,

    /// What ShellCheck said about the problem
    message: String,
}

impl From<Record> for ShellcheckComment {
    /// Turns one comment of the report into a comment of the crate
    ///
    /// ShellCheck and Rakko both count lines and columns from 1, so the
    /// positions stay as ShellCheck wrote them.
    fn from(record: Record) -> Self {
        let Record {
            file,
            line,
            column,
            end_line,
            end_column,
            level,
            code,
            message,
        } = record;

        let span = Span::builder()
            .start(Position::builder().line(line).column(column).build())
            .end(
                Position::builder()
                    .line(end_line)
                    .column(end_column)
                    .build(),
            )
            .build();

        Self::builder()
            .path(file)
            .span(span)
            .level(level)
            .code(code)
            .description(message)
            .build()
    }
}

/// Returns the comments that a report of ShellCheck holds
///
/// # Errors
///
/// Returns the error of a report that is not the object that ShellCheck
/// writes. A reading that tolerated such a report would let a run pass while
/// the comments of the run went unread.
pub(super) fn comments(report: &str) -> Result<Vec<ShellcheckComment>, serde_json::Error> {
    let report: Report = serde_json::from_str(report.trim())?;

    Ok(report
        .comments
        .into_iter()
        .map(ShellcheckComment::from)
        .collect())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;

    use super::*;

    /// A report that ShellCheck 0.11.0 wrote for a script with one problem
    const ONE_COMMENT: &str = r#"{"comments":[{"file":"run.sh","line":2,"endLine":2,"column":6,"endColumn":8,"level":"info","code":2086,"message":"Double quote to prevent globbing and word splitting.","fix":{"replacements":[{"column":6,"endColumn":6,"endLine":2,"insertionPoint":"afterEnd","line":2,"precedence":7,"replacement":"\""}]}}]}"#;

    #[test]
    fn comments_of_a_clean_report_are_empty() {
        let comments = comments("{\"comments\":[]}\n").unwrap();

        assert!(comments.is_empty(), "expected no comment, got {comments:?}");
    }

    #[test]
    fn comments_of_a_report_hold_the_code_of_the_check() {
        let comments = comments(ONE_COMMENT).unwrap();

        assert_eq!(comments.first().map(ShellcheckComment::code), Some(2086));
    }

    #[test]
    fn comments_of_a_report_hold_the_path_of_the_script() {
        let comments = comments(ONE_COMMENT).unwrap();

        assert_eq!(
            comments.first().map(|comment| comment.path().as_path()),
            Some(Path::new("run.sh"))
        );
    }

    #[test]
    fn comments_of_a_report_hold_the_range_of_shellcheck() {
        let comments = comments(ONE_COMMENT).unwrap();

        assert_eq!(
            comments.first().map(ShellcheckComment::span),
            Some(
                Span::builder()
                    .start(Position::builder().line(2).column(6).build())
                    .end(Position::builder().line(2).column(8).build())
                    .build()
            )
        );
    }

    #[test]
    fn comments_of_a_report_with_an_unknown_level_fail() {
        let report = ONE_COMMENT.replace("\"info\"", "\"notice\"");

        let error = comments(&report);

        assert!(
            error.is_err(),
            "expected the reading to fail, got {error:?}"
        );
    }

    #[test]
    fn comments_of_an_empty_report_fail() {
        let error = comments("");

        assert!(
            error.is_err(),
            "expected the reading to fail, got {error:?}"
        );
    }

    #[test]
    fn record_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Record>();
    }

    #[test]
    fn report_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Report>();
    }
}
