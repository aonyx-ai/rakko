use thiserror::Error;

/// An error that leaves a caller without the answer of a run
///
/// A problem that the validator found in a configuration is no failure of a
/// reading: it travels in the report, and the caller decides what it means.
/// The variants here describe a log that holds no answer about the project
/// that this crate can trust.
///
/// The shape of the log belongs to a version of Renovate. A record that does
/// not fit therefore points at a version that this crate does not know, and
/// the findings of such a run would be the problems that the reading happened
/// to understand.
#[derive(Debug, Error)]
pub enum ReadReportError {
    /// The validator wrote a line that is no JSON record
    #[error("failed to read a line of the log of the validator: {line}")]
    MalformedRecord {
        /// The line that the crate could not read
        line: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// The validator reported a migration before it named a configuration
    ///
    /// A record that reports a migration names no file. The record that
    /// announces the configuration before it names the file, and a log
    /// without that announcement leaves the migration without a place.
    #[error("the validator reported a migration of no configuration that it named: {line}")]
    UnattributedMigration {
        /// The record of the migration
        line: String,
    },

    /// The validator wrote a warning or an error that the crate does not know
    #[error("the validator wrote a record that the action does not know: {line}")]
    UnrecognizedRecord {
        /// The record that the crate does not know
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
