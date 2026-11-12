//! The error that leaves a run without an answer

use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of taplo without an answer
///
/// Nothing here is a problem of the project. A file that taplo refused and a
/// file that it would reformat travel as problems of the observation. These
/// variants describe a run whose report cannot be trusted, and the action
/// that asked for the run reports it instead of answering from it.
///
/// A later release can add a variant for another reason why a run of taplo has
/// no answer. So a match on this error outside this crate needs an arm for a
/// variant that it does not know.
///
/// # Examples
///
/// A report that lost its last line keeps what taplo wrote:
///
/// ```
/// use rakko_taplo::ObserveTaploError;
///
/// let error = ObserveTaploError::IncompleteReport {
///     stderr: "error: invalid TOML".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "taplo wrote a report that ends before its last line: error: invalid TOML"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObserveTaploError {
    /// No attempt held an answer, and the last one lost part of the report
    ///
    /// Taplo can lose the end of its report when it exits. A run that ended
    /// without success names at least one problem. So a report of such a run
    /// that names none lost the lines that held them. The run repeats a few
    /// times. This is what remains when no attempt held an answer, and the
    /// report of the last attempt names no problem.
    #[error("taplo wrote a report that ends before its last line: {stderr}")]
    IncompleteReport {
        /// What taplo wrote to its standard error stream
        stderr: String,
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

    /// No attempt held an answer, and the last one left out the configuration
    /// file of the project
    ///
    /// Taplo reports the configuration file that it found, and only then warns
    /// when it rejects that file and runs with its defaults. A report that does
    /// not mention a configuration file, while the project holds one, cannot
    /// show whether taplo rejected it. Either taplo lost the lines, or it did not find the file
    /// and ran with its defaults. The run repeats a few times. This is what
    /// remains when no attempt held an answer, and the report of the last
    /// attempt does not mention the file.
    #[error("taplo did not report the configuration file {}: {stderr}", path.display())]
    UnreportedConfiguration {
        /// The configuration file that the project holds
        path: PathBuf,

        /// What taplo wrote to its standard error stream
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_taplo_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ObserveTaploError>();
    }
}
