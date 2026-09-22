use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::node::ObserveNodeError;
use crate::node::report::ReadReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A test that failed travels as a
/// finding in the outcome of the run. The variants of this error describe a
/// run whose answer cannot be trusted, and such a run stops instead of
/// reporting one.
#[derive(Debug, Error)]
pub enum TestTypeScriptError {
    /// Node reported no failure and ended without success
    ///
    /// The test runner reports every test that it ran and every one that
    /// failed. A run that ended without success and reported no failure
    /// therefore stopped for a reason that the action cannot name, and the
    /// tests of the project never ran to the end.
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

    /// Node wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of Node. A report that
    /// does not fit therefore points at a version that this crate does not
    /// know, and the findings of such a run would be the failures that the
    /// reading happened to understand.
    #[error("node wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What node wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Mise reported no node for the project
    ///
    /// The project pins no Node, or nothing installed the pin yet. The action
    /// installs nothing, so the run stops here.
    #[error("failed to resolve node")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveNodeError> for TestTypeScriptError {
    /// Turns the failure of a run of node into the error of the action
    ///
    /// The machinery that runs node names the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    // testtypescript[impl report.unreadable]
    // testtypescript[impl report.unreported]
    fn from(error: ObserveNodeError) -> Self {
        match error {
            ObserveNodeError::MissingFailure { details } => Self::MissingFailure { details },
            ObserveNodeError::NodeUnavailable { source } => Self::NodeUnavailable { source },
            ObserveNodeError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What node wrote in a run that reported no failure
    const DETAILS: &str = "out of memory";

    /// What node wrote in a run whose report the action could not read
    const REPORT: &str = "v24.20.0";

    // testtypescript[verify report.unreported]
    #[test]
    fn error_of_a_silent_run_holds_what_node_wrote() {
        let error = TestTypeScriptError::from(ObserveNodeError::MissingFailure {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, TestTypeScriptError::MissingFailure { details } if details == DETAILS),
            "expected what node wrote, got {error:?}"
        );
    }

    // testtypescript[verify report.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_node_wrote() {
        let error = TestTypeScriptError::from(ObserveNodeError::UnreadableReport {
            report: REPORT.to_owned(),
            source: ReadReportError::MissingHeader,
        });

        assert!(
            matches!(&error, TestTypeScriptError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of node, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn test_type_script_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<TestTypeScriptError>();
    }
}
