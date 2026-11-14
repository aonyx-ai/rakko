//! The error that leaves a run of Terraform without an answer

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of Terraform without an answer
///
/// A file that is not formatted is no failure of a run. It travels in the list
/// that the run returns, and the caller decides what it means. The variants
/// here describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub(crate) enum ObserveTerraformError {
    /// Terraform did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run Terraform")]
    TerraformUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Terraform could not read a file of the project
    ///
    /// Terraform stops at the first file that it cannot parse, and it writes
    /// diagnostics that say why. The files after it stay unread.
    #[error("Terraform could not read the configuration of the project:\n{diagnostics}")]
    UnreadableConfiguration {
        /// The diagnostics that Terraform wrote
        diagnostics: String,
    },

    /// Terraform wrote a report that the crate does not recognize
    ///
    /// A check that found a file that is not formatted lists it. A check that
    /// ends with the status of such a file and lists none leaves the run
    /// unexplained.
    #[error("Terraform wrote a report that the action does not recognize: {stderr}")]
    UnrecognizedReport {
        /// What Terraform wrote to its standard error stream
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn observe_terraform_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::ObserveTerraformError>();
    }
}
