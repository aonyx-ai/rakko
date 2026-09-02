use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of prettier without an answer
///
/// Nothing here is a problem of the project. A file that prettier refused and
/// a file that it would rewrite travel as problems of the observation. This
/// variant describes a run whose report cannot be trusted, and the action that
/// asked for the run reports it instead of answering from it.
#[derive(Debug, Error)]
pub enum ObservePrettierError {
    /// Prettier did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    ///
    /// Prettier runs on Node, and mise installs the two as separate tools. A
    /// project that pins prettier without Node resolves a program that cannot
    /// start, and the failure arrives here.
    #[error("failed to run prettier")]
    PrettierUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}
