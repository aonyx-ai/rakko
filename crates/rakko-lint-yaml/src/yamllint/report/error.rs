use thiserror::Error;

/// An error that stops the reading of a report of yamllint
///
/// A rule that a file broke is no failure of the reading. The variant here
/// describes a report whose shape this crate does not know, which belongs to
/// a version of yamllint that the project pinned after this crate was
/// written.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ReadReportError {
    /// A line of the report is not a problem that the crate can read
    ///
    /// Yamllint writes one line per problem in the parsable format, and each
    /// line names a file, a line, a column, a level, and a description. A line
    /// that names something else is a line that this crate cannot turn into a
    /// finding.
    #[error("yamllint wrote a line that does not report a problem: {line}")]
    UnreadableLine {
        /// The line that the reading could not turn into a problem
        line: String,
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
