use rakko_action::Name;
use rakko_cli::clawless::event::SendError;
use thiserror::Error;

/// An error that stops a run of the pre-commit command
///
/// A problem of the project is no error of this run. Each action reports what
/// it found, and the run reads those reports and counts them. The variants
/// here describe the three ways in which a run cannot let a commit continue:
/// the actions did not leave the project ready for one, an action asked for a
/// run that the command does not know how to give, or what an action found
/// never reached the reader.
#[derive(Debug, Error)]
pub enum PreCommitError {
    /// Actions of the run found problems in the project, or stopped
    ///
    /// The run reported every action before it, so the message counts them
    /// instead of repeating what they said.
    #[error("{problems} of {total} actions found problems or stopped")]
    FailedActions {
        /// How many actions found problems or stopped
        problems: usize,

        /// How many actions the run drove
        total: usize,
    },

    /// An action declares a kind of access that the command does not know
    ///
    /// A newer release of the contract crate can add a kind of access, and
    /// this release of the command cannot know how to run an action that
    /// declares it. The command sorts every action by its access before the
    /// first one runs, so this error stops the run before any action runs.
    /// An update of Rakko brings a release of the command that knows the kind.
    #[error(
        "the {action} action declares a kind of access that pre-commit does not know; update Rakko to run it"
    )]
    UnknownAccess {
        /// The action that declares the kind of access
        action: Name,
    },

    /// What an action found did not reach the reader
    ///
    /// The run writes each report as it arrives, and it stops at a report that
    /// nothing takes, because the reader would learn nothing about the actions
    /// that follow either.
    #[error("failed to report the outcome of the {action} action")]
    UnreportedOutcome {
        /// The action whose outcome the run could not report
        action: Name,

        /// The cause of the failure
        source: SendError,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::action_name;
    use rakko_cli::clawless::event::Event;

    use super::*;

    #[test]
    fn failed_actions_counts_the_actions_that_did_not_pass() {
        let error = PreCommitError::FailedActions {
            problems: 2,
            total: 12,
        };

        assert_eq!(
            error.to_string(),
            "2 of 12 actions found problems or stopped"
        );
    }

    #[test]
    fn unknown_access_names_the_action_and_the_fix() {
        let error = PreCommitError::UnknownAccess {
            action: action_name!("format-yaml"),
        };

        assert_eq!(
            error.to_string(),
            "the format-yaml action declares a kind of access that pre-commit does not know; update Rakko to run it"
        );
    }

    #[test]
    fn unreported_outcome_names_the_action() {
        let error = PreCommitError::UnreportedOutcome {
            action: action_name!("lint-rust"),
            source: SendError(Event::Message("lost".to_owned())),
        };

        assert_eq!(
            error.to_string(),
            "failed to report the outcome of the lint-rust action"
        );
    }
}
