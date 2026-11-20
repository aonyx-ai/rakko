//! The error that stops a run of the action

use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::git::CompareCommitsError;
use crate::otto::ListUnitsError;
use crate::pull_request::ReadPullRequestError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the pull request. A unit without a changeset
/// travels as a finding in the outcome of the run. Each variant of this error
/// describes a run that has a pull request and cannot judge it. Such a run
/// stops instead of reporting an answer.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_require_changesets::RequireChangesetsError;
///
/// let error = RequireChangesetsError::MissingEventPath;
///
/// assert_eq!(
///     error.to_string(),
///     "the run is in a pull request, but `GITHUB_EVENT_PATH` gives no event file"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RequireChangesetsError {
    /// Otto could not reach a list of the release units
    ///
    /// Otto reaches no list outside a Git repository, for example.
    #[error("otto could not list the release units: {details}")]
    FailedUnitList {
        /// What Otto wrote about the failure
        details: String,
    },

    /// Git did not run
    #[error("failed to run git")]
    GitUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Git cannot compare the base with the head of the pull request
    ///
    /// The checkout lacks a commit of the comparison, or the history that
    /// leads to the last commit that both share. A checkout of depth 1, the
    /// default of `actions/checkout`, has neither, so the workflow checks out
    /// the whole history.
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

    /// The event file holds no pull request that the action reads
    #[error("failed to read the pull request from the event file {}", path.display())]
    MalformedEvent {
        /// The path of the event file
        path: PathBuf,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// The run is in a pull request, and no variable gives its event file
    ///
    /// GitHub Actions gave the name of the event of a pull request in
    /// `GITHUB_EVENT_NAME`, and `GITHUB_EVENT_PATH` is not set. A skip would
    /// hide a check that has something to judge.
    #[error("the run is in a pull request, but `GITHUB_EVENT_PATH` gives no event file")]
    MissingEventPath,

    /// Otto did not run
    #[error("failed to run otto")]
    OttoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Otto stopped at a condition of the repository that an operator repairs
    ///
    /// Two units with the same name are an example. The message of Otto says
    /// what to repair.
    #[error("otto stopped before it listed the release units: {message}")]
    StoppedUnitList {
        /// The message of the stop
        message: String,
    },

    /// Git cannot tell where the project lies in its repository
    ///
    /// The root of the project is in no Git repository, for example.
    #[error("git cannot locate the project in a repository: {details}")]
    UnlocatedProject {
        /// What Git wrote about the failure
        details: String,
    },

    /// The event file cannot be read
    #[error("failed to read the event file {}", path.display())]
    UnreadableEvent {
        /// The path of the event file
        path: PathBuf,

        /// The cause of the failure
        source: std::io::Error,
    },

    /// Otto wrote a list of the release units that the action cannot read
    ///
    /// The shape of the list belongs to a version of Otto, so this points at a
    /// version that this crate does not know.
    #[error("failed to read the list of release units that otto wrote: {output}")]
    UnreadableUnitList {
        /// What Otto wrote in place of a list
        output: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Mise reported no Otto at the version that the action declares
    ///
    /// Mise did not run, or nothing installed that version of Otto yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve otto")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<CompareCommitsError> for RequireChangesetsError {
    /// Turns a comparison that Git did not finish into the error of the action
    fn from(error: CompareCommitsError) -> Self {
        match error {
            CompareCommitsError::GitUnavailable { source } => Self::GitUnavailable { source },
            CompareCommitsError::IncomparableCommits {
                base,
                head,
                details,
            } => Self::IncomparableCommits {
                base,
                head,
                details,
            },
            CompareCommitsError::UnlocatedProject { details } => Self::UnlocatedProject { details },
        }
    }
}

impl From<ListUnitsError> for RequireChangesetsError {
    /// Turns a list that Otto did not give into the error of the action
    fn from(error: ListUnitsError) -> Self {
        match error {
            ListUnitsError::FailedList { details } => Self::FailedUnitList { details },
            ListUnitsError::OttoUnavailable { source } => Self::OttoUnavailable { source },
            ListUnitsError::StoppedList { message } => Self::StoppedUnitList { message },
            ListUnitsError::UnreadableList { output, source } => {
                Self::UnreadableUnitList { output, source }
            }
        }
    }
}

impl From<ReadPullRequestError> for RequireChangesetsError {
    /// Turns a pull request that the action cannot read into the error of the
    /// action
    fn from(error: ReadPullRequestError) -> Self {
        match error {
            ReadPullRequestError::MalformedEvent { path, source } => {
                Self::MalformedEvent { path, source }
            }
            ReadPullRequestError::MissingEventPath => Self::MissingEventPath,
            ReadPullRequestError::UnreadableEvent { path, source } => {
                Self::UnreadableEvent { path, source }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn error_of_a_stopped_list_holds_the_message_of_otto() {
        let error = RequireChangesetsError::from(ListUnitsError::StoppedList {
            message: "two units have the same name".to_owned(),
        });

        assert_eq!(
            error.to_string(),
            "otto stopped before it listed the release units: two units have the same name"
        );
    }

    #[test]
    fn require_changesets_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RequireChangesetsError>();
    }
}
