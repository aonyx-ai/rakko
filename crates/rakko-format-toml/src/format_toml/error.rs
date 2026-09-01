use std::path::PathBuf;

use rakko_taplo::ObserveTaploError;
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A file that is not formatted and
/// a file that taplo cannot parse travel as findings in the outcome of the
/// run. The variants of this error describe a run whose answer cannot be
/// trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum FormatTomlError {
    /// Taplo reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Taplo starts in the root, so this
    /// points at a report that the action misread.
    #[error("taplo reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that taplo reported
        path: PathBuf,
    },

    /// Taplo rejected a configuration file of the project
    ///
    /// Taplo warns about a configuration that it cannot read and then runs
    /// with its defaults. A run on the defaults quietly does what the project
    /// asked it not to do, so the action stops instead of reporting what such
    /// a run found.
    #[error("taplo rejected a configuration file and would have run with its defaults: {details}")]
    RejectedConfiguration {
        /// What taplo reported about the file
        details: String,
    },

    /// Taplo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run taplo")]
    TaploUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Taplo wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of taplo. A run that
    /// ended without success and named no problem, and a run that passed
    /// without the count of the files, both wrote something the action could
    /// not read, and an answer built on such a report would hide problems
    /// behind a green result.
    #[error("taplo wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What taplo wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no taplo for the project
    ///
    /// The project pins no taplo, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve taplo")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveTaploError> for FormatTomlError {
    /// Turns the failure of a taplo run into the error of the action
    ///
    /// The machinery that runs taplo names the same two conditions that the
    /// action reports, so the conversion renames them and adds nothing. A
    /// report that never arrived complete is one that the action cannot
    /// read, whatever kept it from arriving.
    fn from(error: ObserveTaploError) -> Self {
        match error {
            ObserveTaploError::IncompleteReport { stderr } => Self::UnrecognizedReport { stderr },
            ObserveTaploError::TaploUnavailable { source } => Self::TaploUnavailable { source },
        }
    }
}
