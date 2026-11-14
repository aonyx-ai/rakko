//! The error that stops a run of the action

use std::path::PathBuf;

use aonyx_rakko_shfmt::ObserveShfmtError;
use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A file that is not formatted and
/// a script that shfmt cannot parse travel as findings in the outcome of the
/// run. The variants of this error describe a run whose answer cannot be
/// trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_format_shell::FormatShellError;
///
/// let error = FormatShellError::UnrecognizedReport {
///     stderr: "open run.sh: permission denied".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "shfmt wrote a report that the action does not recognize: open run.sh: permission denied"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FormatShellError {
    /// Shfmt reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Shfmt walks the root, so this points
    /// at a report that the action misread.
    #[error("shfmt reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that shfmt reported
        path: PathBuf,
    },

    /// Shfmt did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run shfmt")]
    ShfmtUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// A run of shfmt failed for a reason that the action does not know
    ///
    /// A later release of the machinery that runs shfmt can report a new kind
    /// of failure. The action carries that failure as the source, so the
    /// message still says what went wrong.
    #[error("failed to get an answer from shfmt")]
    UnknownShfmtFailure {
        /// The cause of the failure
        source: ObserveShfmtError,
    },

    /// Shfmt wrote a report that the action does not recognize
    ///
    /// Shfmt writes an error that it found in a file with the position of the
    /// error. An error without a position, such as a file that shfmt cannot
    /// open, means that shfmt did not examine every file. An answer from such
    /// a report can hide a problem behind a green result, so the run stops
    /// instead.
    #[error("shfmt wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What shfmt wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no shfmt at the version that the action declares
    ///
    /// Mise did not run, or nothing installed that version of shfmt yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve shfmt")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveRequirementError,
    },
}

impl From<ObserveShfmtError> for FormatShellError {
    /// Turns the failure of a shfmt run into the error of the action
    ///
    /// The machinery that runs shfmt names the same two conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    ///
    /// A failure that a later release of the machinery adds becomes the source
    /// of a variant of its own.
    fn from(error: ObserveShfmtError) -> Self {
        match error {
            ObserveShfmtError::ShfmtUnavailable { source } => Self::ShfmtUnavailable { source },
            ObserveShfmtError::UnrecognizedReport { stderr } => Self::UnrecognizedReport { stderr },
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

    /// What shfmt wrote in a run that the machinery could not read
    const REPORT: &str = "open run.sh: permission denied\n";

    #[test]
    fn error_of_an_unrecognized_report_holds_what_shfmt_wrote() {
        let error = FormatShellError::from(ObserveShfmtError::UnrecognizedReport {
            stderr: REPORT.to_owned(),
        });

        assert!(
            matches!(&error, FormatShellError::UnrecognizedReport { stderr } if stderr == REPORT),
            "expected the report of shfmt, got {error:?}"
        );
    }

    #[test]
    fn format_shell_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatShellError>();
    }
}
