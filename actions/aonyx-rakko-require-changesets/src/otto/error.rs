//! The error of a list of release units that Otto did not give

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves the action without the release units of the
/// repository
///
/// Without the units, the check cannot tell which files belong to which unit,
/// so the run stops.
#[derive(Debug, Error)]
pub(crate) enum ListUnitsError {
    /// Otto could not reach a list, such as outside a Git repository
    #[error("otto could not list the release units: {details}")]
    FailedList {
        /// What Otto wrote about the failure
        details: String,
    },

    /// Otto did not run
    #[error("failed to run otto")]
    OttoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Otto stopped at a condition of the repository that an operator repairs,
    /// such as two units with the same name
    #[error("otto stopped before it listed the release units: {message}")]
    StoppedList {
        /// The message of the stop, which says what to repair
        message: String,
    },

    /// Otto wrote a list that the action cannot read
    #[error("failed to read the list of release units that otto wrote: {output}")]
    UnreadableList {
        /// What Otto wrote in place of a list
        output: String,

        /// The cause of the failure
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_units_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ListUnitsError>();
    }
}
