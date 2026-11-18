//! The error that stops a run of the action

use std::path::PathBuf;

use aonyx_rakko_shfmt::ObserveShfmtError;
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::shellcheck::ObserveShellcheckError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A problem that ShellCheck found
/// in a script travels as a finding in the outcome of the run. The variants of
/// this error describe a run whose answer cannot be trusted, and such a run
/// stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_lint_shell::LintShellError;
///
/// let error = LintShellError::IncompleteExamination {
///     details: "run.sh: Permission denied".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "ShellCheck stopped before it examined every script: run.sh: Permission denied"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LintShellError {
    /// A tool reported a path that the project root does not contain
    ///
    /// A finding gives its path relative to the project root, and a path
    /// outside the root has no such form. Shfmt searches the root, and the run
    /// gives ShellCheck the scripts that shfmt found. So this points at a
    /// report that the action misread.
    #[error("a tool reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that the tool reported
        path: PathBuf,
    },

    /// ShellCheck stopped before it examined every script
    ///
    /// ShellCheck examines the scripts that it can open, and it writes a line
    /// about each one that it cannot open. It also stops when it does not
    /// accept its command line. Either way the report describes less than the
    /// scripts of the project. An outcome built on it would hide the rest
    /// behind that part.
    #[error("ShellCheck stopped before it examined every script: {details}")]
    IncompleteExamination {
        /// What ShellCheck wrote about the run that it stopped
        details: String,
    },

    /// ShellCheck did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run ShellCheck")]
    ShellcheckUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Shfmt did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. The run found no script to examine.
    #[error("failed to run shfmt")]
    ShfmtUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// A search of shfmt failed for a reason that the action does not know
    ///
    /// A later release of the machinery that runs shfmt can report a new kind
    /// of failure. The action carries that failure as the source, so the
    /// message still says what went wrong.
    #[error("failed to get an answer from shfmt")]
    UnknownShfmtFailure {
        /// The cause of the failure
        source: ObserveShfmtError,
    },

    /// ShellCheck wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of ShellCheck. A report
    /// that the action cannot read therefore points at a version that it does
    /// not know. The findings of such a run would be the ones that the reading
    /// happened to understand.
    #[error("failed to read the report of ShellCheck: {report}")]
    UnreadableReport {
        /// What ShellCheck wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Shfmt wrote a report that the action does not recognize
    ///
    /// Shfmt opens each file and each directory during a search. It writes a
    /// line about one that it cannot open, and it stops the search there. A
    /// list of scripts from such a search can be short. A run over a short
    /// list can hide a problem behind a green result, so the run stops.
    #[error("shfmt wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedShfmtReport {
        /// What shfmt wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no ShellCheck at the version that the action declares
    ///
    /// Mise did not run, or nothing installed that version of ShellCheck yet.
    /// The action installs nothing, so the run stops here. The action can also
    /// declare no ShellCheck, declare it at two versions, or name a program
    /// that ShellCheck does not provide. No change of the project repairs
    /// that.
    #[error("failed to resolve ShellCheck")]
    UnresolvedShellcheck {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no shfmt at the version that the action declares
    ///
    /// Mise did not run, or nothing installed that version of shfmt yet. The
    /// action installs nothing, so the run stops here. The action can also
    /// declare no shfmt, declare it at two versions, or name a program that
    /// shfmt does not provide. No change of the project repairs that.
    #[error("failed to resolve shfmt")]
    UnresolvedShfmt {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveShellcheckError> for LintShellError {
    /// Turns the failure of a ShellCheck run into the error of the action
    ///
    /// The machinery that runs ShellCheck gives the same three conditions that
    /// the action reports, so the conversion renames them and adds nothing.
    fn from(error: ObserveShellcheckError) -> Self {
        match error {
            ObserveShellcheckError::IncompleteExamination { details } => {
                Self::IncompleteExamination { details }
            }
            ObserveShellcheckError::ShellcheckUnavailable { source } => {
                Self::ShellcheckUnavailable { source }
            }
            ObserveShellcheckError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
        }
    }
}

impl From<ObserveShfmtError> for LintShellError {
    /// Turns the failure of a shfmt search into the error of the action
    ///
    /// The machinery that runs shfmt gives the same two conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    ///
    /// A failure that a later release of the machinery adds becomes the source
    /// of a variant of its own.
    fn from(error: ObserveShfmtError) -> Self {
        match error {
            ObserveShfmtError::ShfmtUnavailable { source } => Self::ShfmtUnavailable { source },
            ObserveShfmtError::UnrecognizedReport { stderr } => {
                Self::UnrecognizedShfmtReport { stderr }
            }
            source => Self::UnknownShfmtFailure { source },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// What ShellCheck wrote about a script that it could not open
    const DETAILS: &str = "run.sh: run.sh: openBinaryFile: permission denied";

    /// What shfmt wrote about a script that it could not open
    const SEARCH: &str = "open secret.sh: permission denied\n";

    #[test]
    fn error_of_an_incomplete_examination_holds_what_shellcheck_wrote() {
        let error = LintShellError::from(ObserveShellcheckError::IncompleteExamination {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, LintShellError::IncompleteExamination { details } if details == DETAILS),
            "expected what ShellCheck wrote, got {error:?}"
        );
    }

    #[test]
    fn error_of_an_unrecognized_search_holds_what_shfmt_wrote() {
        let error = LintShellError::from(ObserveShfmtError::UnrecognizedReport {
            stderr: SEARCH.to_owned(),
        });

        assert!(
            matches!(&error, LintShellError::UnrecognizedShfmtReport { stderr } if stderr == SEARCH),
            "expected what shfmt wrote, got {error:?}"
        );
    }

    #[test]
    fn lint_shell_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintShellError>();
    }
}
