use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::markdownlint::ObserveMarkdownlintError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A rule that a file broke travels
/// as a finding in the outcome of the run. The variants of this error
/// describe a run whose answer cannot be trusted, and such a run stops
/// instead of reporting one.
#[derive(Debug, Error)]
pub enum LintMarkdownError {
    /// Markdownlint reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Markdownlint starts in the root, so
    /// this points at a report that the action misread.
    #[error("markdownlint reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that markdownlint reported
        path: PathBuf,
    },

    /// Markdownlint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run markdownlint")]
    MarkdownlintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Markdownlint wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of markdownlint. A run
    /// that could not open a file also ends this way, because markdownlint
    /// then writes the failure of its runtime in place of the report, and an
    /// answer built on such a report would leave an unknown part of the
    /// project unread.
    #[error("markdownlint wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What markdownlint wrote in place of a report
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Markdownlint ended without success and reported no rule
    ///
    /// A run that failed says why in its report, so a failure without one
    /// happened for a reason that the action cannot name, and an answer built
    /// on it would hide every problem behind a green result.
    #[error("markdownlint ended without success and reported nothing: {stderr}")]
    UnrecognizedReport {
        /// What markdownlint wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no markdownlint for the project
    ///
    /// The project pins no markdownlint, or nothing installed the pin yet.
    /// The action installs nothing, so the run stops here.
    #[error("failed to resolve markdownlint")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveMarkdownlintError> for LintMarkdownError {
    /// Turns the failure of a markdownlint run into the error of the action
    ///
    /// The machinery that runs markdownlint names the same two conditions
    /// that the action reports, so the conversion renames them and adds
    /// nothing.
    // lintmarkdown[impl check.unreadable]
    fn from(error: ObserveMarkdownlintError) -> Self {
        match error {
            ObserveMarkdownlintError::MarkdownlintUnavailable { source } => {
                Self::MarkdownlintUnavailable { source }
            }
            ObserveMarkdownlintError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that builds an
    // unreadable report expects the reading to fail. A `# Panics` section on
    // every test would repeat that and give the reader no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What markdownlint wrote in a run that the crate could not read
    const REPORT: &str = "Error: EACCES: permission denied, open 'locked.md'\n";

    // lintmarkdown[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_markdownlint_wrote() {
        let source = serde_json::from_str::<Vec<u8>>(REPORT)
            .expect_err("the test builds a report that is not JSON");

        let error = LintMarkdownError::from(ObserveMarkdownlintError::UnreadableReport {
            report: REPORT.to_owned(),
            source,
        });

        assert!(
            matches!(&error, LintMarkdownError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of markdownlint, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn lint_markdown_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintMarkdownError>();
    }
}
