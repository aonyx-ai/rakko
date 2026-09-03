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
#[derive(Debug, Error)]
pub enum FormatYamlError {
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
    /// Prettier ignores an option that it does not know with a warning and
    /// then runs without it, and it reports a configuration file that it
    /// cannot read at all. A run that lost part of the configuration quietly
    /// does what the project asked it not to do, so the action stops instead
    /// of reporting what such a run found.
    #[error("prettier did not run with the configuration of the project: {details}")]
    RejectedConfiguration {
        /// What prettier reported about the configuration
        details: String,
    },

    /// Prettier wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of prettier. A run that
    /// ended without success and named no problem wrote something the action
    /// could not read, and an answer built on such a report would hide
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

impl From<ObservePrettierError> for FormatYamlError {
    /// Turns the failure of a prettier run into the error of the action
    ///
    /// The machinery that runs prettier names the one condition that the
    /// action reports, so the conversion renames it and adds nothing.
    fn from(error: ObservePrettierError) -> Self {
        match error {
            ObservePrettierError::PrettierUnavailable { source } => {
                Self::PrettierUnavailable { source }
            }
        }
    }
}
