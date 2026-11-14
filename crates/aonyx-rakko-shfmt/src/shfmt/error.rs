//! The error that leaves a run without an answer

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of shfmt without an answer
///
/// Nothing here is a problem of the project. A file that is not formatted and
/// a script that shfmt cannot parse travel in the observation. These variants
/// describe a run whose report cannot be trusted, and the action that asked
/// for the run reports it instead of answering from it.
///
/// A later release can add a variant for another reason why a run of shfmt has
/// no answer. So a match on this error outside this crate needs an arm for a
/// variant that it does not know.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_shfmt::ObserveShfmtError;
///
/// let error = ObserveShfmtError::UnrecognizedReport {
///     stderr: "open run.sh: permission denied".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "shfmt wrote a report that the crate does not recognize: open run.sh: permission denied"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObserveShfmtError {
    /// Shfmt did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run shfmt")]
    ShfmtUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Shfmt wrote a report that the crate does not recognize
    ///
    /// Shfmt writes an error that it found in a file with the position of the
    /// error. An error without a position, such as a file or a directory that
    /// shfmt cannot open, means that shfmt did not examine every file. A run
    /// that ended without success and reported nothing has no answer either.
    /// An answer from such a report can hide a problem behind a green result,
    /// so the report stops the run instead.
    #[error("shfmt wrote a report that the crate does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What shfmt wrote to its standard error stream
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_shfmt_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ObserveShfmtError>();
    }
}
