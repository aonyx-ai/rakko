use clawless::event::SendError;
use rakko_action::Name;
use thiserror::Error;

/// An error that stops a run of a group
///
/// A problem of the project is no error of the run. Each member reports what
/// it found, and the scheduler reads those reports and counts them. The
/// variants here describe the three ways in which the scheduler fails a run of
/// the members that the command line bound:
///
/// - members found problems in the project, or stopped,
/// - a member declares a kind of access that the scheduler cannot order, or
/// - what a member found never reached the reader.
#[derive(Debug, Error)]
pub(crate) enum RunGroupError {
    /// Members of the group found problems in the project, or stopped
    ///
    /// The run reported every member before it, so the message counts them
    /// instead of repeating what they said.
    #[error("{problems} of {total} actions found problems or stopped")]
    FailedMembers {
        /// How many members found problems or stopped
        problems: usize,

        /// How many members the run drove
        total: usize,
    },

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
    /// The run writes each report when its member finishes, and it stops at a
    /// report that nothing takes, because the reader would learn nothing about
    /// the members that follow either.
    #[error("failed to report the outcome of the {member} action")]
    UnreportedOutcome {
        /// The member whose outcome the run could not report
        member: Name,

        /// The cause of the failure
        source: SendError,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use clawless::event::Event;
    use rakko_action::action_name;

    use super::*;

    #[test]
    fn failed_members_counts_the_members_that_did_not_pass() {
        let error = RunGroupError::FailedMembers {
            problems: 2,
            total: 12,
        };

        assert_eq!(
            error.to_string(),
            "2 of 12 actions found problems or stopped"
        );
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
}
