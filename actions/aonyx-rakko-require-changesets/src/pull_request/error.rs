//! The error of a pull request that the action cannot read

use std::path::PathBuf;

use thiserror::Error;

/// An error that stops the action before it knows its pull request
///
/// The run is in a pull request, because GitHub Actions gave the name of such
/// an event. A skip here would hide a check that has something to judge, so
/// the run stops instead.
#[derive(Debug, Error)]
pub(crate) enum ReadPullRequestError {
    /// The variable that gives the path of the event file is not set
    #[error("the run is in a pull request, but `GITHUB_EVENT_PATH` gives no event file")]
    MissingEventPath,

    /// The event file holds no pull request that the action reads
    #[error("failed to read the pull request from the event file {}", path.display())]
    MalformedEvent {
        /// The path of the event file
        path: PathBuf,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// The event file cannot be read
    #[error("failed to read the event file {}", path.display())]
    UnreadableEvent {
        /// The path of the event file
        path: PathBuf,

        /// The cause of the failure
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_pull_request_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ReadPullRequestError>();
    }
}
