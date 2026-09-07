use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::yamllint::report::ReadReportError;

/// An error that leaves a run of yamllint without an answer
///
/// A rule that a file broke is no failure of a run: it travels in the
/// observation, and the caller decides what it means. The variants here
/// describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub enum ObserveYamllintError {
    /// Yamllint wrote a report that the crate cannot read
    ///
    /// The shape of a line belongs to a version of yamllint. A report that
    /// holds a line which names no problem therefore points at a version that
    /// this crate does not know, and the findings of such a run would be the
    /// lines that the reading happened to understand.
    #[error("yamllint wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What yamllint wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Yamllint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run yamllint")]
    YamllintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
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
    fn observe_yamllint_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveYamllintError>();
    }
}
