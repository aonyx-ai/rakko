//! The error that stops a run of a group

use clawless::event::SendError;
use rakko_action::Name;
use thiserror::Error;

/// An error that stops a run of a group
///
/// A problem of the project is no error of the run. Each member reports what
/// it found, and the code of the run tells a member that found problems from a
/// member that stopped. The variants here describe the two ways in which the
/// scheduler fails a run of the members that the command line bound:
///
/// - a member declares a kind of access that the scheduler cannot order, or
/// - what the members found never reached the reader.
#[derive(Debug, Error)]
pub(crate) enum RunGroupError {
    /// A member declares a kind of access that this version of the command
    /// line does not know
    ///
    /// A later release of Rakko can add a kind of access. The scheduler cannot
    /// tell in which order or in which place a run of such a kind belongs. A
    /// run in the project can change files that the declaration does not let
    /// it change. So the run stops before any member runs. The message tells
    /// the user to update Rakko, because only a newer command line can know
    /// the kind.
    #[error(
        "the {member} action declares a kind of access that this command line does not know; update Rakko to run it"
    )]
    UnknownAccess {
        /// The member whose declaration the scheduler does not know
        member: Name,
    },

    /// What a member found did not reach the reader
    ///
    /// The run writes each report when its member finishes. It stops at a
    /// report that nothing takes, because the reader would learn nothing about
    /// the members that follow either.
    #[error("failed to report the outcome of the {member} action")]
    UnreportedOutcome {
        /// The member whose outcome the run could not report
        member: Name,

        /// The cause of the failure
        source: SendError,
    },

    /// The summary of the group did not reach the reader
    ///
    /// The run sends the summary when every member finished. Every member
    /// already reported, so only the summary is lost. But the summary is the
    /// one artifact of the run, and a machine reads nothing else, so the run
    /// fails.
    #[error("failed to report the summary of the {group} group")]
    UnreportedSummary {
        /// The group whose summary the run could not report
        group: Name,

        /// The cause of the failure
        source: SendError,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use clawless::event::Event;
    use rakko_action::action_name;

    use super::*;

    #[test]
    fn run_group_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RunGroupError>();
    }

    #[test]
    fn unknown_access_tells_the_user_to_update_rakko() {
        let error = RunGroupError::UnknownAccess {
            member: action_name!("format-sdk"),
        };

        assert_eq!(
            error.to_string(),
            "the format-sdk action declares a kind of access that this command line does not know; update Rakko to run it"
        );
    }

    #[test]
    fn unreported_outcome_gives_the_member() {
        let error = RunGroupError::UnreportedOutcome {
            member: action_name!("lint-rust"),
            source: SendError(Event::Message("lost".to_owned())),
        };

        assert_eq!(
            error.to_string(),
            "failed to report the outcome of the lint-rust action"
        );
    }

    #[test]
    fn unreported_summary_gives_the_group() {
        let error = RunGroupError::UnreportedSummary {
            group: action_name!("pre-commit"),
            source: SendError(Event::Message("lost".to_owned())),
        };

        assert_eq!(
            error.to_string(),
            "failed to report the summary of the pre-commit group"
        );
    }
}
