use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::validator::report::ReadReportError;

/// An error that leaves a run of the validator without an answer
///
/// A problem that the validator found in a configuration is no failure of a
/// run: it travels in the observation, and the caller decides what it means.
/// The variants here describe a run that produced no answer about the project
/// that the caller can trust.
#[derive(Debug, Error)]
pub enum ObserveValidatorError {
    /// The validator stopped before it had validated the project
    ///
    /// The validator ends with a status of its own when it crashes, and it
    /// writes a fatal record when it cannot read the global configuration that
    /// a run of Renovate would start with. It validated less than the project
    /// in both cases, so an answer built on such a run would describe a part
    /// of the project as though it were the whole.
    #[error("the validator stopped before it had validated the project: {details}")]
    AbortedValidation {
        /// What the validator wrote about the run that it stopped
        details: String,
    },

    /// The validator reported no problem and ended without success
    ///
    /// The validator reports every problem that it finds before it fails a
    /// run. A run that failed and said nothing therefore failed for a reason
    /// that this crate cannot name.
    #[error("the validator reported no problem and ended without success: {details}")]
    MissingReport {
        /// What the validator wrote in place of a problem
        details: String,
    },

    /// The validator wrote a log that the crate cannot read
    #[error("failed to read the log of the validator: {report}")]
    UnreadableReport {
        /// What the validator wrote in place of a log that the crate reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// The validator did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was validated.
    #[error("failed to run renovate-config-validator")]
    ValidatorUnavailable {
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

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn observe_validator_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveValidatorError>();
    }
}
