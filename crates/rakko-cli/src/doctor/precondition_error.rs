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
/// without the check. The pins of the toolchains are no part of it, so an
/// action that a toolchain pin does not satisfy stops no other run.
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
    /// The report holds each problem of the generated file and of the pins
    /// that override a declared tool. The message gives the command that
    /// repairs them. A toolchain is no part of the precondition, so the
    /// report gives no pin of a toolchain.
    #[error(
        "the tool pins of the project do not meet the precondition of the mounted actions, so \
         the run stops. `mise run rakko -- doctor --fix` repairs the generated file and each pin \
         that overrides a declared tool:\n{report}"
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
