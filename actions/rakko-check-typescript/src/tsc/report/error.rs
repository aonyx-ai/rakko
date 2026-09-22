use thiserror::Error;

/// An error that leaves a caller without the diagnostics of a run
///
/// A rule of the language that the code breaks is no failure of a reading: it
/// travels as a diagnostic, and the caller decides what it means. The variant
/// here describes output that holds no answer about the project.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Error)]
pub enum ReadReportError {
    /// Tsc wrote a line that is no part of a diagnostic
    ///
    /// The shape of the report belongs to a version of tsc. A line that does
    /// not fit therefore points at a version that this crate does not know,
    /// and the findings of such a run would be the diagnostics that the
    /// reading happened to understand.
    #[error("failed to read a line of the report of tsc: {line}")]
    UnreadableLine {
        /// The line that the crate could not read
        line: String,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn read_report_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadReportError>();
    }
}
