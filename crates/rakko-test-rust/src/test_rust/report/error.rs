use thiserror::Error;

/// An error that stops the reading of a report of nextest
///
/// The reading tolerates every line that is not a record of nextest, so the
/// JSON of cargo can share the stream. A record of nextest that the crate
/// cannot read is another matter: the shape belongs to a version of nextest,
/// and a reading that skipped a test it did not understand would let a run
/// pass with its failures unread. The reading therefore stops, and the action
/// reports the error.
#[derive(Debug, Error)]
pub enum ReadNextestReportError {
    /// Nextest wrote a record whose kind the crate knows and whose body it
    /// cannot read
    ///
    /// The line is about a test or a binary of tests, and its fields differ
    /// from the shape that the crate expects.
    #[error("nextest wrote a record that the action cannot read: {line}")]
    UnrecognizedRecord {
        /// The line as nextest wrote it
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
    fn read_nextest_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadNextestReportError>();
    }
}
