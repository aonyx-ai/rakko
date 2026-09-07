use thiserror::Error;

/// An error that stops the reading of a report of cargo-udeps
///
/// The reading tolerates every line that is not the report, because cargo
/// writes its own records to the same stream. The report itself is another
/// matter: its shape belongs to a version of cargo-udeps, and a reading that
/// skipped a report it did not understand would let a workspace pass with
/// its unused dependencies unread. The reading therefore stops, and the
/// action that asked for it reports the error.
#[derive(Debug, Error)]
pub enum ReadUdepsReportError {
    /// Cargo-udeps wrote its report in a shape that the crate cannot read
    ///
    /// The line names the unused dependencies of the run, and its fields
    /// differ from the shape that the crate expects.
    #[error("cargo-udeps wrote a report that the crate cannot read: {line}")]
    UnrecognizedReport {
        /// The line as cargo-udeps wrote it
        line: String,

        /// The cause of the failure
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn read_udeps_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadUdepsReportError>();
    }
}
