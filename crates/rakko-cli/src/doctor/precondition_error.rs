//! What the command line reports when the precondition on the tools stops a
//! run

use thiserror::Error;

use super::check_error::CheckToolPinsError;
use crate::report::Report;

/// An error that occurs when the command line checks the precondition on the
/// tools before a run
///
/// The command line checks the tool pins of the project before it runs an
/// action, a command that the harness wrote, or a group. Each variant stops
/// the run before it starts, also in CI. Only the help and `doctor` run
/// without the check.
#[derive(Debug, Error)]
pub(crate) enum CheckPreconditionError {
    /// The check of the tool pins did not get an answer
    ///
    /// The run cannot know whether the project meets the precondition, so it
    /// stops as it does for a project that does not.
    #[error(
        "failed to check the tool pins of the project, so the run stops until the cause below \
         is repaired"
    )]
    UncheckedToolPins {
        /// The cause of the failure
        source: CheckToolPinsError,
    },

    /// The tool pins of the project do not meet the precondition
    ///
    /// The report holds each problem that a run of `doctor` gives. The
    /// message gives the command that repairs them, and the report gives the
    /// pin of each toolchain that the project must change itself.
    #[error(
        "the tool pins of the project do not meet the precondition of the mounted actions, so \
         the run stops. `mise run rakko -- doctor --fix` repairs the generated file and each pin \
         that overrides a declared tool. It does not change the pin of a toolchain, so change \
         each such pin as the report says:\n{report}"
    )]
    UnmetPrecondition {
        /// The report of `doctor` with each problem
        report: Report,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// The error that stops a run is readable across threads
    ///
    /// The command line gives the error to the runner of Clawless, which can
    /// carry it to another thread.
    #[test]
    fn check_precondition_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckPreconditionError>();
    }
}
