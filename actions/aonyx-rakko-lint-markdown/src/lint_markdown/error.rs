//! The error that stops a run of the action

use std::io::Error as IoError;
use std::path::PathBuf;

use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

use crate::markdownlint::ObserveMarkdownlintError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A rule that a file broke travels
/// as a finding in the outcome of the run. The variants of this error
/// describe a run whose answer cannot be trusted, and such a run stops
/// instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_lint_markdown::LintMarkdownError;
/// use std::path::PathBuf;
///
/// let error = LintMarkdownError::ForeignPath {
///     path: PathBuf::from("/outside/project/file.md"),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "markdownlint reported a path outside the project: /outside/project/file.md"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
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

    /// Markdownlint cannot parse the configuration of the project
    ///
    /// Markdownlint treats a configuration file that it cannot parse as a file
    /// that is not there. It then runs on another configuration file or on its
    /// defaults, and it says nothing. Such a run does what the project asked
    /// it not to do. So the action stops instead of reporting what the
    /// run found.
    #[error("markdownlint cannot parse the configuration that starts in {}: {details}", path.display())]
    RejectedConfiguration {
        /// The first configuration file of markdownlint that exists in the
        /// project root, relative to that root
        ///
        /// The configuration starts in this file. The file that markdownlint
        /// cannot parse can also be a file that this file extends.
        path: PathBuf,

        /// What markdownlint wrote about the configuration, which gives the
        /// file that it cannot parse
        details: String,
    },

    /// Markdownlint wrote a report that the action cannot read
    ///
    /// The report shape belongs to a version of markdownlint. A run that could
    /// not open a file also ends this way. Markdownlint then writes the failure
    /// of its runtime in place of the report. So an answer built on such a
    /// report would leave an unknown part of the project unread.
    #[error("failed to read markdownlint's report: {report}")]
    UnreadableReport {
        /// What markdownlint wrote in place of a report
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Markdownlint ended without success and reported no rule
    ///
    /// A run that failed says why in its report. So a failure without one
    /// happened for a reason that the action cannot name. An answer built on it
    /// would hide every problem behind a green result.
    #[error("markdownlint ended without success and reported nothing: {stderr}")]
    UnrecognizedReport {
        /// What markdownlint wrote to its standard error stream
        stderr: String,
    },

    /// The action cannot write the temporary configuration that extends the
    /// file of the project
    ///
    /// The run gives markdownlint a temporary configuration file that extends
    /// the configuration file of the project. Without it, markdownlint does
    /// not report a configuration that it cannot parse. The write fails when
    /// the file system refuses the temporary file. It also fails when the
    /// root is relative and the current directory is unknown. It fails when
    /// the path of the configuration file is not valid Unicode. Nothing of the
    /// project was examined.
    #[error(
        "failed to write the temporary markdownlint configuration that extends the file of the project"
    )]
    UnwritableReference {
        /// The cause of the failure
        source: IoError,
    },

    /// Mise reported no markdownlint at the version that the action declares, or
    /// no Node that the action accepts
    ///
    /// Mise did not run, or the project pins no Node in the range of the
    /// action. Nothing installed the declared markdownlint or the pinned Node
    /// yet, or the directory of that Node cannot be an entry of `PATH`. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve markdownlint")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveRequirementError,
    },
}

impl From<ObserveMarkdownlintError> for LintMarkdownError {
    /// Turns the failure of a markdownlint run into the error of the action
    ///
    /// The machinery that runs markdownlint reports the same conditions as
    /// the action, so the conversion renames them and adds nothing.
    fn from(error: ObserveMarkdownlintError) -> Self {
        match error {
            ObserveMarkdownlintError::MarkdownlintUnavailable { source } => {
                Self::MarkdownlintUnavailable { source }
            }
            ObserveMarkdownlintError::RejectedConfiguration { path, details } => {
                Self::RejectedConfiguration { path, details }
            }
            ObserveMarkdownlintError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
            ObserveMarkdownlintError::UnwritableReference { source } => {
                Self::UnwritableReference { source }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and a test that builds an unreadable report expects the reading to fail. A `# Panics` section on every test would repeat that and give the reader no information."
    )]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What markdownlint wrote in a run that the crate could not read
    const REPORT: &str = "Error: EACCES: permission denied, open 'locked.md'\n";

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

    #[test]
    fn error_of_an_unwritable_reference_holds_its_cause() {
        let source = IoError::other("the disk is full");

        let error =
            LintMarkdownError::from(ObserveMarkdownlintError::UnwritableReference { source });

        assert!(
            matches!(&error, LintMarkdownError::UnwritableReference { source } if source.to_string() == "the disk is full"),
            "expected the cause of the failure, got {error:?}"
        );
    }

    /// Shows that error is Send and Sync
    ///
    /// An action puts the error in the outcome of a run, and that outcome holds
    /// an error that another thread can read. This test holds the error to the
    /// auto traits that make this possible. This matters because a field of a
    /// later version could take them away without a word from the compiler.
    #[test]
    fn lint_markdown_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintMarkdownError>();
    }

    #[test]
    fn lint_markdown_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintMarkdownError>();
    }
}
