//! The error that leaves a run of ShellCheck without an answer

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of ShellCheck without an answer
///
/// A problem that ShellCheck found in a script is no failure of a run. It
/// travels as a comment, and the caller decides what it means. The variants
/// here describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub(crate) enum ObserveShellcheckError {
    /// ShellCheck stopped before it examined every script
    ///
    /// ShellCheck examines the scripts that it can open, and it writes a line
    /// about each one that it cannot open. It also stops when it does not
    /// accept its command line. Either way the report describes less than the
    /// scripts of the project.
    #[error("ShellCheck stopped before it examined every script: {details}")]
    IncompleteExamination {
        /// What ShellCheck wrote about the run that it stopped
        details: String,
    },

    /// ShellCheck did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run ShellCheck")]
    ShellcheckUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// ShellCheck wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of ShellCheck. A report
    /// that this crate cannot read therefore points at a version that it does
    /// not know. The findings of such a run would be the ones that the reading
    /// happened to understand.
    #[error("failed to read the report of ShellCheck: {report}")]
    UnreadableReport {
        /// What ShellCheck wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn observe_shellcheck_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::ObserveShellcheckError>();
    }
}
