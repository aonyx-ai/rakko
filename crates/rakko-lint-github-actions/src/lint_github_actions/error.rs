use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::zizmor::ObserveZizmorError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A pattern that zizmor recognized
/// in a workflow travels as a finding in the outcome of the run. The variants
/// of this error describe a run whose answer cannot be trusted, and such a run
/// stops instead of reporting one.
#[derive(Debug, Error)]
pub enum LintGitHubActionsError {
    /// Zizmor reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. A run names the root as the place to
    /// look, so this points at a report that the action misread.
    #[error("zizmor reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that zizmor reported
        path: PathBuf,
    },

    /// Zizmor stopped before it had audited the project
    ///
    /// Zizmor reads the configuration of the project before it collects a
    /// single file, and it audits nothing when it cannot accept that
    /// configuration. It also stops at a file that it collected and cannot
    /// read, because the run asks it to instead of letting the file leave the
    /// audit through a warning. Either way the report describes less than the
    /// project, and an outcome built on it would hide the rest behind that
    /// part.
    #[error("zizmor stopped before it had audited the project: {details}")]
    IncompleteAudit {
        /// What zizmor wrote about the run that it stopped
        details: String,
    },

    /// Zizmor wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of zizmor. A report that
    /// the action cannot read therefore points at a version that this crate
    /// does not know, and the findings of such a run would be the ones that
    /// the reading happened to understand.
    #[error("zizmor wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What zizmor wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Mise reported no zizmor for the project
    ///
    /// The project pins no zizmor, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve zizmor")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Zizmor did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was audited.
    #[error("failed to run zizmor")]
    ZizmorUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}

impl From<ObserveZizmorError> for LintGitHubActionsError {
    /// Turns the failure of a zizmor run into the error of the action
    ///
    /// The machinery that runs zizmor names the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    // lintgithubactions[impl check.configuration]
    // lintgithubactions[impl check.incomplete]
    // lintgithubactions[impl check.unreadable]
    fn from(error: ObserveZizmorError) -> Self {
        match error {
            ObserveZizmorError::IncompleteAudit { details } => Self::IncompleteAudit { details },
            ObserveZizmorError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
            ObserveZizmorError::ZizmorUnavailable { source } => Self::ZizmorUnavailable { source },
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that builds the
    // cause of a failure expects the reading to fail. A `# Panics` section on
    // every test would repeat that and give the reader no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What zizmor wrote about a configuration that it refused
    const DETAILS: &str = "error: configuration error in .";

    /// What zizmor wrote in a run whose report the crate could not read
    const REPORT: &str = "fatal: no audit was performed";

    // lintgithubactions[verify check.configuration]
    // lintgithubactions[verify check.incomplete]
    #[test]
    fn error_of_a_run_that_stopped_holds_what_zizmor_wrote() {
        let error = LintGitHubActionsError::from(ObserveZizmorError::IncompleteAudit {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, LintGitHubActionsError::IncompleteAudit { details } if details == DETAILS),
            "expected what zizmor wrote, got {error:?}"
        );
    }

    // lintgithubactions[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_zizmor_wrote() {
        let source = serde_json::from_str::<Vec<u8>>(REPORT)
            .expect_err("the test reads a report that is no report");
        let error = LintGitHubActionsError::from(ObserveZizmorError::UnreadableReport {
            report: REPORT.to_owned(),
            source,
        });

        assert!(
            matches!(&error, LintGitHubActionsError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of zizmor, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn lint_github_actions_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintGitHubActionsError>();
    }
}
