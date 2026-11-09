//! The error that stops a run of the action

use std::path::PathBuf;

use rakko_prettier::ObservePrettierError;
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A file that is not formatted and
/// a file that prettier cannot parse travel as findings in the outcome of the
/// run. The variants of this error describe a run whose answer cannot be
/// trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use rakko_format_markdown::FormatMarkdownError;
///
/// let error = FormatMarkdownError::ForeignPath {
///     path: PathBuf::from("/etc/passwd"),
/// };
/// assert_eq!(
///     error.to_string(),
///     "prettier reported a path outside the project: /etc/passwd"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FormatMarkdownError {
    /// Prettier reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Prettier starts in the root, so this
    /// points at a report that the action misread.
    #[error("prettier reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that prettier reported
        path: PathBuf,
    },

    /// Prettier did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run prettier")]
    PrettierUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// A configuration of the project did not reach the run
    ///
    /// Prettier ignores an unknown option with a warning and runs without it.
    /// It reports a configuration file that it cannot read at all. A run that
    /// lost part of the configuration quietly does what the project asked it
    /// not to do. So the action stops instead of reporting what such a run
    /// found.
    #[error("prettier did not run with the configuration of the project: {details}")]
    RejectedConfiguration {
        /// What prettier reported about the configuration
        details: String,
    },

    /// A run of prettier failed for a reason that the action does not know
    ///
    /// A later release of the machinery that runs prettier can report a new
    /// kind of failure. The action carries that failure as the source, so the
    /// message still says what went wrong.
    #[error("failed to get an answer from prettier")]
    UnknownPrettierFailure {
        /// The cause of the failure
        source: ObservePrettierError,
    },

    /// Prettier wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of prettier.
    /// A run that ended without success and named no problem wrote something
    /// the action could not read. An answer built on such a report would hide
    /// problems behind a green result.
    #[error("prettier wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What prettier wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no prettier for the project
    ///
    /// The project pins no prettier, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve prettier")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObservePrettierError> for FormatMarkdownError {
    /// Turns the failure of a prettier run into the error of the action
    ///
    /// The machinery that runs prettier names the one condition that the
    /// action reports, so the conversion renames it and adds nothing.
    ///
    /// A failure that a later release of the machinery adds becomes the source
    /// of a variant of its own.
    fn from(error: ObservePrettierError) -> Self {
        match error {
            ObservePrettierError::PrettierUnavailable { source } => {
                Self::PrettierUnavailable { source }
            }
            source => Self::UnknownPrettierFailure { source },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_markdown_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatMarkdownError>();
    }
}
