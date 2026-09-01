use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of taplo without an answer
///
/// Nothing here is a problem of the project. A file that taplo refused and a
/// file that it would reformat travel as problems of the observation. These
/// variants describe a run whose report cannot be trusted, and the action
/// that asked for the run reports it instead of answering from it.
#[derive(Debug, Error)]
pub enum ObserveTaploError {
    /// Every attempt lost part of the report
    ///
    /// Taplo can lose the tail of its report when it exits, and the problems
    /// that a report without its closing line holds can be incomplete. The
    /// run repeats a few times, and this is what remains when no attempt
    /// answered completely.
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
}
