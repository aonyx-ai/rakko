use thiserror::Error;

/// An error that stops the reading of a report of cargo
///
/// The reading tolerates every line that is not a record of cargo, so the
/// output of a tool that cargo runs can share the stream. A record of cargo
/// that the crate cannot read is another matter: the shape belongs to a
/// version of cargo, and a reading that skipped a diagnostic it did not
/// understand would let a build pass with its problems unread. The reading
/// therefore stops, and the action that asked for it reports the error.
#[derive(Debug, Error)]
pub enum ReadReportError {
    /// Cargo wrote a record whose reason the crate knows and whose body it
    /// cannot read
    ///
    /// The line names a compiler message or the end of the build, and its
    /// fields differ from the shape that the crate expects.
    #[error("cargo wrote a record that the crate cannot read: {line}")]
    UnrecognizedRecord {
        /// The line as cargo wrote it
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
    fn read_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadReportError>();
    }
}
