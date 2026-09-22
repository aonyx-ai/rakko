use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of oxfmt without an answer
///
/// A problem of the project is not one of these. A file that is not formatted
/// and a file that oxfmt could not format both travel in the observation of
/// the run.
#[derive(Debug, Error)]
pub enum ObserveOxfmtError {
    /// Oxfmt did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run oxfmt")]
    OxfmtUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}
