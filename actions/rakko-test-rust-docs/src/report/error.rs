//! The error that stops the reading

use thiserror::Error;

/// An error that stops the reading of what the test harness reported
///
/// Nothing here is a problem of the project. An example that failed travels
/// as a finding in the outcome of the run. This error describes a report
/// that the crate cannot read, and a run that answered from such a report
/// would count fewer examples than ran.
///
/// # Examples
///
/// ```
/// use rakko_test_rust_docs::ReadDoctestReportError;
///
/// let error = ReadDoctestReportError::UnreadableSummary {
///     line: "invalid summary".to_string(),
/// };
///
/// assert_eq!(error.to_string(), "the summary of the examples has no count of passed and failed examples: invalid summary");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Error)]
pub enum ReadDoctestReportError {
    /// A summary of the harness does not count the examples
    ///
    /// The harness closes the examples of a package with a summary. The
    /// summary counts what passed and what failed. The shape of that line
    /// belongs to a version of the harness.
    #[error("the summary of the examples has no count of passed and failed examples: {line}")]
    UnreadableSummary {
        /// The summary that the crate cannot read
        line: String,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Shows that `ReadDoctestReportError` is `Send` and `Sync`
    ///
    /// An action puts the error in the outcome of a run, and that outcome
    /// holds an error that another thread can read. This test holds the error
    /// to the auto traits that make this possible. A field of a later version
    /// could take them away without a word from the compiler.
    #[test]
    fn read_doctest_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadDoctestReportError>();
    }

    #[test]
    fn read_doctest_report_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ReadDoctestReportError>();
    }
}
