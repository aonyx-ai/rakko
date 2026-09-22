use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::node::report::ReadReportError;

/// An error that leaves a run of the test runner without an answer
///
/// A test that failed is no failure of a run: it travels in the report, and
/// the caller decides what it means. The variants here describe a run that
/// produced no answer about the project at all.
#[derive(Debug, Error)]
pub enum ObserveNodeError {
    /// The runner reported no failure and ended without success
    ///
    /// The runner reports every test that it ran and every one that failed, so
    /// a run that ended without success and reported no failure stopped for a
    /// reason that this crate cannot name, and the tests of the project never
    /// ran to the end.
    #[error("node reported no failure and ended without success: {details}")]
    MissingFailure {
        /// What node wrote in place of a failure
        details: String,
    },

    /// Node did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was tested.
    #[error("failed to run node")]
    NodeUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Node wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of Node. A report that
    /// does not fit therefore points at a version that this crate does not
    /// know, and the failures of such a run would be the ones that the reading
    /// happened to understand.
    #[error("node wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What node wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },
}

impl ObserveNodeError {
    /// Returns the error of a run that ended without success and found nothing
    ///
    /// The runner writes its report on the standard output stream, so that is
    /// what the details carry. A run that stopped before it wrote one leaves
    /// its reason on the standard error stream, or says nothing at all.
    // testtypescript[impl report.unreported]
    pub(super) fn silent(stdout: &str, stderr: &str) -> Self {
        Self::MissingFailure {
            details: diagnosis(stdout, stderr),
        }
    }
}

/// Returns what node wrote about a run that reported no failure
///
/// The standard error stream answers first, because a run that stopped for a
/// reason of its own writes that reason there, while the standard output
/// stream carries the report that said nothing. A run that wrote nothing there
/// leaves only the report.
fn diagnosis(stdout: &str, stderr: &str) -> String {
    let written = stderr.trim();

    if written.is_empty() {
        return stdout.trim().to_owned();
    }

    written.to_owned()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // testtypescript[verify report.unreported]
    #[test]
    fn error_of_a_silent_run_holds_what_node_wrote() {
        let error = ObserveNodeError::silent("", "out of memory");

        assert!(
            matches!(&error, ObserveNodeError::MissingFailure { details } if details == "out of memory"),
            "expected what node wrote, got {error:?}"
        );
    }

    // testtypescript[verify report.unreported]
    #[test]
    fn error_of_a_silent_run_that_wrote_nothing_holds_the_other_stream() {
        let error = ObserveNodeError::silent("TAP version 13", "");

        assert!(
            matches!(&error, ObserveNodeError::MissingFailure { details } if details == "TAP version 13"),
            "expected what node wrote, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn observe_node_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveNodeError>();
    }
}
