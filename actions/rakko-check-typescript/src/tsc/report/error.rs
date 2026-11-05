//! The error that leaves a caller without the diagnostics of a run

use thiserror::Error;

/// An error that leaves a caller without the diagnostics of a run
///
/// A rule of the language that the code breaks is no failure of a reading. It
/// travels as a diagnostic, and the caller decides what it means. The variant
/// here describes output that holds no answer about the project.
///
/// # Examples
///
/// ```
/// use rakko_check_typescript::ReadReportError;
///
/// let error = ReadReportError::UnreadableLine {
///     line: "unexpected output".to_owned(),
/// };
/// assert_eq!(
///     error.to_string(),
///     "tsc wrote a line that is no part of a diagnostic: unexpected output"
/// );
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Error)]
pub enum ReadReportError {
    /// Tsc wrote a line that is no part of a diagnostic
    ///
    /// The shape of the report belongs to a version of tsc. A line that does
    /// not fit therefore points at a version that this crate does not know. The
    /// findings of such a run would be the diagnostics that the reading
    /// happened to understand.
    #[error("tsc wrote a line that is no part of a diagnostic: {line}")]
    UnreadableLine {
        /// The line that the crate could not read
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

    /// Shows that the error is send and sync
    ///
    /// An action puts the error in the outcome of a run, and that outcome holds
    /// an error that another thread can read. This test holds the error to the
    /// auto traits that make this possible. It does so because a field of a
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
