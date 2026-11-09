//! The error that leaves a run without an answer

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of prettier without an answer
///
/// Nothing here is a problem of the project. A file that prettier refused and
/// a file that it would rewrite travel as problems of the observation. This
/// variant describes a run whose report cannot be trusted, and the action that
/// asked for the run reports it instead of answering from it.
///
/// A later release can add a variant for another reason why a run of prettier
/// has no answer. So a match on this error outside this crate needs an arm for
/// a variant that it does not know.
///
/// # Examples
///
/// A program that cannot start leaves the run without an answer:
///
/// ```
/// use rakko_prettier::ObservePrettierError;
/// use rakko_tool::Invocation;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let source = Invocation::new("rakko-no-such-program").run().await.unwrap_err();
/// let error = ObservePrettierError::PrettierUnavailable { source };
///
/// assert_eq!(error.to_string(), "failed to run prettier");
/// # }
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ObservePrettierError {
    /// Prettier did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run prettier")]
    PrettierUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_prettier_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ObservePrettierError>();
    }
}
