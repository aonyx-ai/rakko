use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::oxfmt::ObserveOxfmtError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A file that is not formatted and
/// a file that oxfmt cannot parse travel as findings in the outcome of the
/// run. The variants of this error describe a run whose answer cannot be
/// trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum FormatTypeScriptError {
    /// Oxfmt reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Oxfmt starts in the root, so this
    /// points at a report that the action misread.
    #[error("oxfmt reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that oxfmt reported
        path: PathBuf,
    },

    /// Oxfmt did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run oxfmt")]
    OxfmtUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Oxfmt refused a configuration file of the project
    ///
    /// Oxfmt reads its configuration before it collects a single file, and it
    /// formats nothing at all when it cannot accept that configuration, so the
    /// project asked for rules that never applied.
    #[error("oxfmt did not run with the configuration of the project: {details}")]
    RejectedConfiguration {
        /// What oxfmt reported about the configuration
        details: String,
    },

    /// Oxfmt wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of oxfmt. A run that ended
    /// without success and named no problem wrote something the action could
    /// not read, and an answer built on such a report would hide problems
    /// behind a green result.
    #[error("oxfmt wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What oxfmt wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no oxfmt for the project
    ///
    /// The project pins no oxfmt, or nothing installed the pin yet. The action
    /// installs nothing, so the run stops here.
    #[error("failed to resolve oxfmt")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveOxfmtError> for FormatTypeScriptError {
    /// Turns the failure of an oxfmt run into the error of the action
    ///
    /// The machinery that runs oxfmt names the one condition that the action
    /// reports, so the conversion renames it and adds nothing.
    fn from(error: ObserveOxfmtError) -> Self {
        match error {
            ObserveOxfmtError::OxfmtUnavailable { source } => Self::OxfmtUnavailable { source },
        }
    }
}
