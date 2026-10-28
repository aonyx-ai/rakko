use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::stiles::ObserveStilesError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A diagnostic that a rule
/// reported travels as a finding in the outcome of the run. The variants of
/// this error describe a run whose answer cannot be trusted, and such a run
/// stops instead of reporting one.
#[derive(Debug, Error)]
pub enum LintStilesError {
    /// Stiles reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. A run names the root as the place to
    /// check, so this points at a report that the action misread.
    #[error("stiles reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that stiles reported
        path: PathBuf,
    },

    /// Stiles stopped before it had checked the project
    ///
    /// Stiles stops when it cannot read its configuration, when it cannot
    /// build or load a lint that the configuration names, and when it cannot
    /// read or analyze a file that it found. The report of such a run
    /// describes less than the project, and an outcome built on it would hide
    /// the rest behind that part.
    #[error("stiles stopped before it had checked the project: {details}")]
    IncompleteCheck {
        /// What stiles wrote about the run that it stopped
        details: String,
    },

    /// Stiles did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was checked.
    #[error("failed to run stiles")]
    StilesUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Stiles wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of stiles. A report that
    /// the action cannot read therefore points at a version that this crate
    /// does not know, and the findings of such a run would be the ones that
    /// the reading happened to understand.
    #[error("stiles wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What stiles wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Mise reported no stiles for the project
    ///
    /// The project pins no stiles, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve stiles")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveStilesError> for LintStilesError {
    /// Turns the failure of a stiles run into the error of the action
    ///
    /// The machinery that runs stiles names the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    fn from(error: ObserveStilesError) -> Self {
        match error {
            ObserveStilesError::IncompleteCheck { details } => Self::IncompleteCheck { details },
            ObserveStilesError::StilesUnavailable { source } => Self::StilesUnavailable { source },
            ObserveStilesError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
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

    /// What stiles wrote about a configuration that it refused
    const DETAILS: &str = "Error: failed to load the stiles configuration";

    /// What stiles wrote in a run whose report the crate could not read
    const REPORT: &str = "src/lib.rs:7:5: warning";

    #[test]
    fn error_of_a_run_that_stopped_holds_what_stiles_wrote() {
        let error = LintStilesError::from(ObserveStilesError::IncompleteCheck {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, LintStilesError::IncompleteCheck { details } if details == DETAILS),
            "expected what stiles wrote, got {error:?}"
        );
    }

    #[test]
    fn error_of_an_unreadable_report_holds_what_stiles_wrote() {
        let source = serde_json::from_str::<Vec<u8>>(REPORT)
            .expect_err("the test reads a report that is no report");
        let error = LintStilesError::from(ObserveStilesError::UnreadableReport {
            report: REPORT.to_owned(),
            source,
        });

        assert!(
            matches!(&error, LintStilesError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of stiles, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn lint_stiles_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintStilesError>();
    }
}
