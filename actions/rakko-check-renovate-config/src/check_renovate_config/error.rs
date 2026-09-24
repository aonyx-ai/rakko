use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::validator::ObserveValidatorError;
use crate::validator::report::ReadReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. An option that Renovate would
/// refuse travels as a finding in the outcome of the run. The variants of this
/// error describe a run whose answer cannot be trusted, and such a run stops
/// instead of reporting one.
#[derive(Debug, Error)]
pub enum CheckRenovateConfigError {
    /// The validator stopped before it had validated the project
    ///
    /// The validator ends with a status of its own when it crashes, and it
    /// writes a fatal record when it cannot read the global configuration that
    /// a run of Renovate would start with. It validated less than the project
    /// in both cases, and an outcome built on such a run would hide the rest
    /// behind that part.
    #[error("the validator stopped before it had validated the project: {details}")]
    AbortedValidation {
        /// What the validator wrote about the run that it stopped
        details: String,
    },

    /// The validator reported a configuration outside the project
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. The validator starts in the root, so
    /// this points at a global configuration that the environment of the run
    /// named elsewhere, or at a log that the action misread.
    #[error("the validator reported a configuration outside the project: {}", path.display())]
    ForeignPath {
        /// The path that the validator reported
        path: PathBuf,
    },

    /// The validator reported no problem and ended without success
    ///
    /// The validator reports every problem that it finds before it fails a
    /// run, so a run that failed and said nothing failed for a reason that the
    /// action cannot name.
    #[error("the validator reported no problem and ended without success: {details}")]
    MissingReport {
        /// What the validator wrote in place of a problem
        details: String,
    },

    /// The validator wrote a log that the action cannot read
    ///
    /// The shape of the log belongs to a version of Renovate. A log that the
    /// action cannot read therefore points at a version that this crate does
    /// not know, and the findings of such a run would be the ones that the
    /// reading happened to understand.
    #[error("failed to read the log of the validator: {report}")]
    UnreadableReport {
        /// What the validator wrote in place of a log that the action reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Mise reported no validator for the project
    ///
    /// The project pins no Renovate, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve renovate-config-validator")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
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

impl From<ObserveValidatorError> for CheckRenovateConfigError {
    /// Turns the failure of a run of the validator into the error of the action
    ///
    /// The machinery that runs the validator names the same conditions that
    /// the action reports, so the conversion renames them and adds nothing.
    // checkrenovateconfig[impl check.aborted]
    // checkrenovateconfig[impl check.unreadable]
    // checkrenovateconfig[impl check.unreported]
    fn from(error: ObserveValidatorError) -> Self {
        match error {
            ObserveValidatorError::AbortedValidation { details } => {
                Self::AbortedValidation { details }
            }
            ObserveValidatorError::MissingReport { details } => Self::MissingReport { details },
            ObserveValidatorError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
            ObserveValidatorError::ValidatorUnavailable { source } => {
                Self::ValidatorUnavailable { source }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What the validator wrote about a run that it stopped
    const DETAILS: &str = "Could not parse config file: Unexpected end of input";

    /// What the validator wrote in a run whose log the action could not read
    const REPORT: &str = "{\"level\":40,\"msg\":\"Something new happened\"}";

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn check_renovate_config_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckRenovateConfigError>();
    }

    // checkrenovateconfig[verify check.aborted]
    #[test]
    fn error_of_a_run_that_stopped_holds_what_the_validator_wrote() {
        let error = CheckRenovateConfigError::from(ObserveValidatorError::AbortedValidation {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, CheckRenovateConfigError::AbortedValidation { details } if details == DETAILS),
            "expected what the validator wrote, got {error:?}"
        );
    }

    // checkrenovateconfig[verify check.unreported]
    #[test]
    fn error_of_a_silent_failure_holds_what_the_validator_wrote() {
        let error = CheckRenovateConfigError::from(ObserveValidatorError::MissingReport {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, CheckRenovateConfigError::MissingReport { details } if details == DETAILS),
            "expected what the validator wrote, got {error:?}"
        );
    }

    // checkrenovateconfig[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_log_holds_what_the_validator_wrote() {
        let error = CheckRenovateConfigError::from(ObserveValidatorError::UnreadableReport {
            report: REPORT.to_owned(),
            source: ReadReportError::UnrecognizedRecord {
                line: REPORT.to_owned(),
            },
        });

        assert!(
            matches!(&error, CheckRenovateConfigError::UnreadableReport { report, .. } if report == REPORT),
            "expected the log of the validator, got {error:?}"
        );
    }
}
