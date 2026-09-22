use thiserror::Error;

/// An error that leaves a caller without the report of a run
///
/// A test that failed is no failure of the reading: it arrives as a result of
/// the report, and the caller decides what it means. The variants here
/// describe text that is not the report of a run at all.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Error)]
pub enum ReadReportError {
    /// The text does not start with the line that opens a TAP report
    ///
    /// Every report opens with the version of TAP that follows. Text without
    /// it was written by something other than the reporter that the run asked
    /// for, so there is no report to read.
    #[error("the report does not open with a version of TAP")]
    MissingHeader,

    /// The report ends without the counts of the run
    ///
    /// The reporter closes a report with one line per count, and the counts
    /// say how many tests ran and how many of them failed. A report without
    /// them was cut off, and a reading of it would report the failures that
    /// arrived before the cut as the whole answer.
    #[error("the report ends without the counts of the run")]
    MissingCounts,

    /// A line of the report is no part of the report that this crate reads
    ///
    /// The shape of a report belongs to a version of Node. A line that does
    /// not fit therefore points at a version that this crate does not know,
    /// and the results of such a reading would be the ones that it happened to
    /// understand.
    #[error("the report holds a line that the action cannot read: {line}")]
    UnreadableLine {
        /// The line that the reading could not place
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
