//! The error of a comparison that Git did not finish

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves the action without the files that a pull request
/// changes
#[derive(Debug, Error)]
pub(crate) enum CompareCommitsError {
    /// Git did not run
    #[error("failed to run git")]
    GitUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Git cannot compare the base with the head of the pull request
    ///
    /// The checkout lacks a commit of the comparison, or the commits that lead
    /// to the last commit that they share. A checkout of depth 1, the default
    /// of `actions/checkout`, holds neither.
    #[error(
        "git cannot compare the base {base} with the head {head} of the pull request: {details}"
    )]
    IncomparableCommits {
        /// The head commit of the base branch
        base: String,

        /// The head commit of the pull request
        head: String,

        /// What Git wrote about the failure
        details: String,
    },

    /// Git cannot tell where the project lies in its repository, such as in a
    /// directory that no repository holds
    #[error("git cannot locate the project in a repository: {details}")]
    UnlocatedProject {
        /// What Git wrote about the failure
        details: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_commits_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CompareCommitsError>();
    }
}
