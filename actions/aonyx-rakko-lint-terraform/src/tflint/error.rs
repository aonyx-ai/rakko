//! The error that leaves a run of TFLint without an answer

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of TFLint without an answer
///
/// An issue that a rule found is no failure of a run. It travels in the list
/// that the run returns, and the caller decides what it means. The variants
/// here describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub(crate) enum ObserveTflintError {
    /// TFLint could not inspect every module
    ///
    /// TFLint reports an error for a module that it cannot load, such as a
    /// module with a file that Terraform cannot parse. The other modules can
    /// still have issues, but the report describes less than the project.
    #[error("TFLint could not inspect every module:\n{messages}")]
    IncompleteInspection {
        /// What TFLint wrote about each module that it could not inspect
        messages: String,
    },

    /// TFLint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run TFLint")]
    TflintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// TFLint wrote a report that the crate does not recognize
    ///
    /// The shape of the report belongs to a version of TFLint. A report that
    /// this crate cannot read points at a version that it does not know. A
    /// run that failed with no issue and no error in its report also lands
    /// here, because the report does not explain it.
    #[error("TFLint wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What TFLint wrote to its standard error stream
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn observe_tflint_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::ObserveTflintError>();
    }
}
