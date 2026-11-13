//! The error that stops the reading

use thiserror::Error;

/// An error that stops the reading of a report of cargo
///
/// The reading tolerates every line that is not a record of cargo, so the
/// output of a tool that cargo runs can share the stream. A record of cargo
/// that the crate cannot read is another matter. The shape belongs to a
/// version of cargo. A reading that skipped a diagnostic it did not understand
/// would let a build pass with its problems unread. The reading therefore
/// stops, and the action that asked for it reports the error.
///
/// A later release can add a variant for another reason why a report of cargo
/// cannot be read. So a match on this error outside this crate needs an arm for
/// a variant that it does not know.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::{CargoReport, ReadReportError};
///
/// let error = CargoReport::read(r#"{"reason":"build-finished"}"#).unwrap_err();
///
/// assert!(matches!(error, ReadReportError::UnrecognizedRecord { .. }));
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadReportError {
    /// Cargo wrote a record whose reason the crate knows and whose body it
    /// cannot read
    ///
    /// The line names a compiler message or the end of the build, and its
    /// fields differ from the shape that the crate expects.
    #[error("failed to read the record from cargo: {line}")]
    UnrecognizedRecord {
        /// The line as cargo wrote it
        line: String,

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

    /// Shows that the error type satisfies Send and Sync
    ///
    /// An action puts the error in the outcome of a run, and that outcome
    /// holds an error that another thread can read. This test holds the error
    /// to the auto traits that make this possible. It does so because a field
    /// of a later version could take them away without a word from the
    /// compiler.
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
