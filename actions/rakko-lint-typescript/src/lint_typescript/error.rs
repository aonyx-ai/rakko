use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::oxlint::ObserveOxlintError;
use crate::oxlint::report::ReadReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A rule that a file broke travels
/// as a finding in the outcome of the run. The variants of this error describe
/// a run whose answer cannot be trusted, and such a run stops instead of
/// reporting one.
#[derive(Debug, Error)]
pub enum LintTypeScriptError {
    /// Oxlint reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. A run names the root as the place to
    /// look, so this points at a report that the action misread.
    #[error("oxlint reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that oxlint reported
        path: PathBuf,
    },

    /// Oxlint wrote no report
    ///
    /// Oxlint reports whenever it linted the project, and it writes no report
    /// when it refuses the configuration of the project, because it reads that
    /// configuration before it collects a single file. The project asked for
    /// rules that never applied, so the run stops here.
    #[error("oxlint wrote no report: {details}")]
    MissingReport {
        /// What oxlint wrote in place of a report
        details: String,
    },

    /// Oxlint did not run
    ///
    /// The program was resolved and did not start, or it started and its output
    /// could not be read. Nothing of the project was examined.
    #[error("failed to run oxlint")]
    OxlintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Oxlint wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of oxlint. A report that
    /// does not fit therefore points at a version that this crate does not
    /// know, and the findings of such a run would be the diagnostics that the
    /// reading happened to understand.
    #[error("oxlint wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What oxlint wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Mise reported no oxlint for the project
    ///
    /// The project pins no oxlint, or nothing installed the pin yet. The action
    /// installs nothing, so the run stops here.
    #[error("failed to resolve oxlint")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveOxlintError> for LintTypeScriptError {
    /// Turns the failure of an oxlint run into the error of the action
    ///
    /// The machinery that runs oxlint names the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    // linttypescript[impl check.unreadable]
    // linttypescript[impl check.unreported]
    fn from(error: ObserveOxlintError) -> Self {
        match error {
            ObserveOxlintError::MissingReport { details } => Self::MissingReport { details },
            ObserveOxlintError::OxlintUnavailable { source } => Self::OxlintUnavailable { source },
            ObserveOxlintError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
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

    /// What oxlint wrote about a configuration that it refused
    const DETAILS: &str = "Failed to parse oxlint configuration file.";

    /// What oxlint wrote in a run whose report the action could not read
    const REPORT: &str = r#"{"diagnostics": "none"}"#;

    // linttypescript[verify check.unreported]
    #[test]
    fn error_of_an_absent_report_holds_what_oxlint_wrote() {
        let error = LintTypeScriptError::from(ObserveOxlintError::MissingReport {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, LintTypeScriptError::MissingReport { details } if details == DETAILS),
            "expected what oxlint wrote, got {error:?}"
        );
    }

    // linttypescript[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_oxlint_wrote() {
        let error = LintTypeScriptError::from(ObserveOxlintError::UnreadableReport {
            report: REPORT.to_owned(),
            source: ReadReportError::Absent,
        });

        assert!(
            matches!(&error, LintTypeScriptError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of oxlint, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn lint_type_script_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintTypeScriptError>();
    }
}
