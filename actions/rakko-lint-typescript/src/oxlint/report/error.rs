//! The error that leaves a caller without the report of a run

use thiserror::Error;

/// An error that leaves a caller without the report of a run
///
/// A diagnostic of oxlint is no failure of a reading, even one about the
/// project itself. It travels in the observation. The caller decides what
/// it means. The variants here describe output that holds no answer about the
/// project.
///
/// # Examples
///
/// ```
/// use rakko_lint_typescript::oxlint::report::ReadReportError;
///
/// let error = ReadReportError::Absent;
/// assert_eq!(error.to_string(), "oxlint wrote no report");
/// ```
#[derive(Debug, Error)]
pub enum ReadReportError {
    /// Oxlint wrote no report
    ///
    /// Oxlint writes its report whenever it linted the project, even for a
    /// project where it found no file to lint. Output without one therefore
    /// belongs to a run that ended before it linted anything, such as a run
    /// whose configuration oxlint refused.
    #[error("oxlint wrote no report")]
    Absent,

    /// Oxlint wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of oxlint. A report that
    /// does not fit therefore points at a version that this crate does not
    /// know. The diagnostics of such a run would be the ones that the reading
    /// happened to understand.
    #[error("failed to read the report of oxlint")]
    Unreadable {
        /// The cause of the failure
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Shows that error is Send and Sync
    ///
    /// An action puts the error in the outcome of a run, and that outcome holds
    /// an error that another thread can read. This test holds the error to the
    /// auto traits that make this possible. This matters because a field of a
    /// later version could take them away without a word from the compiler.
    #[test]
    fn read_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadReportError>();
    }

    #[test]
    fn read_report_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ReadReportError>();
    }
}
