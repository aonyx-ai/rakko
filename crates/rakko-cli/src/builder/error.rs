use rakko_action::Name;
use thiserror::Error;

/// An error that occurs when the command line runs an action or a command
///
/// The runner of Clawless prepares a run before it gives the run to the action
/// or the command. A failure in that preparation ends the run before Rakko
/// gets control. The runner reports such a failure only as text, so the
/// error carries that text. The command line also refuses to run an action
/// that declares a kind of access that the command line does not know.
#[derive(Debug, Error)]
pub(crate) enum RunEntryError {
    /// An action declares a kind of access that this version of the command
    /// line does not know
    ///
    /// A later release of Rakko can add a kind of access. The command line
    /// does not know what such a kind asks of the run. A run in the project
    /// can change files that the declaration does not let it change. So the
    /// action does not run, and the run ends as a run whose action stopped.
    /// The message tells the user to update Rakko, because only a newer
    /// command line can know the kind.
    #[error(
        "the {action} action declares a kind of access that this command line does not know; update Rakko to run it"
    )]
    UnknownAccess {
        /// The action whose declaration the command line does not know
        action: Name,
    },

    /// The runner ended the run before the action or the command started
    ///
    /// The runner could not build the context of the run, or it could not
    /// start the runtime that drives the run. Neither failure depends on the
    /// project, so the message gives the reason that the runner stated.
    #[error("the run ended before it started: {reason}")]
    UnstartedRun {
        /// The reason that the runner gave, with every cause
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::action_name;

    use super::*;

    #[test]
    fn unknown_access_tells_the_user_to_update_rakko() {
        let error = RunEntryError::UnknownAccess {
            action: action_name!("check-latest-deps"),
        };

        assert_eq!(
            error.to_string(),
            "the check-latest-deps action declares a kind of access that this command line does not know; update Rakko to run it"
        );
    }
}
