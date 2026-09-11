use rakko_action::Name;
use rakko_cli::clawless::event::SendError;
use thiserror::Error;

/// An error that stops a run of the pre-commit command
///
/// A problem of the project is no error of this run. Each action reports what
/// it found, and the run reads those reports and counts them. The variants
/// here describe the two ways in which a run cannot let a commit continue:
/// the actions did not leave the project ready for one, or what an action
/// found never reached the reader.
#[derive(Debug, Error)]
pub enum PreCommitError {
    /// Actions of the run found problems in the project, or stopped
    ///
    /// The run reported every action before it, so the message counts them
    /// instead of repeating what they said.
    // precommit[impl result.failed]
    #[error("{problems} of {total} actions found problems or stopped")]
    FailedActions {
        /// How many actions found problems or stopped
        problems: usize,

        /// How many actions the run drove
        total: usize,
    },

    /// What an action found did not reach the reader
    ///
    /// The run writes each report as it arrives, and it stops at a report that
    /// nothing takes, because the reader would learn nothing about the actions
    /// that follow either.
    // precommit[impl report.unreported]
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

    // precommit[verify result.failed]
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

    // precommit[verify report.unreported]
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
