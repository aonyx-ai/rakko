use thiserror::Error;

/// An error that leaves a caller without the report of a run
///
/// A rule that a file broke is no failure of a reading: it travels in the
/// observation, and the caller decides what it means. The variants here
/// describe output that holds no answer about the project.
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
    /// know, and the diagnostics of such a run would be the ones that the
    /// reading happened to understand.
    #[error("failed to read the report of oxlint")]
    Unreadable {
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
