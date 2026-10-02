use thiserror::Error;

/// An error that occurs when the command line runs an action or a command
///
/// The runner of Clawless prepares a run before it gives the run to the action
/// or the command. A failure in that preparation ends the run before Rakko
/// gets control. The runner reports such a failure only as text, so the
/// error carries that text.
#[derive(Debug, Error)]
pub(crate) enum RunEntryError {
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
