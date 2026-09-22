use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::tsc::ObserveTscError;
use crate::tsc::report::ReadReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A rule of the language that the
/// code breaks travels as a finding in the outcome of the run. The variants of
/// this error describe a run whose answer cannot be trusted, and such a run
/// stops instead of reporting one.
#[derive(Debug, Error)]
pub enum CheckTypeScriptError {
    /// Tsc reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. A run names the root as the project,
    /// so this points at a report that the action misread.
    #[error("tsc reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that tsc reported
        path: PathBuf,
    },

    /// Tsc reported nothing and ended without success
    ///
    /// Tsc reports a diagnostic for everything that it finds, including for a
    /// configuration that it refuses and for a project that it finds no
    /// configuration of. A run that ended without success and said nothing
    /// therefore stopped for a reason that the action cannot name, and the
    /// project was never checked.
    #[error("tsc reported nothing and ended without success: {details}")]
    MissingReport {
        /// What tsc wrote in place of a diagnostic
        details: String,
    },

    /// Tsc did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was checked.
    #[error("failed to run tsc")]
    TscUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Tsc wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of tsc. A report that does
    /// not fit therefore points at a version that this crate does not know,
    /// and the findings of such a run would be the diagnostics that the
    /// reading happened to understand.
    #[error("tsc wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What tsc wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Mise reported no tsc for the project
    ///
    /// The project pins no TypeScript, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve tsc")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<ObserveTscError> for CheckTypeScriptError {
    /// Turns the failure of a tsc run into the error of the action
    ///
    /// The machinery that runs tsc names the same three conditions that the
    /// action reports, so the conversion renames them and adds nothing.
    // checktypescript[impl check.unreadable]
    // checktypescript[impl check.unreported]
    fn from(error: ObserveTscError) -> Self {
        match error {
            ObserveTscError::MissingReport { details } => Self::MissingReport { details },
            ObserveTscError::TscUnavailable { source } => Self::TscUnavailable { source },
            ObserveTscError::UnreadableReport { report, source } => {
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

    /// What tsc wrote in a run that reported no diagnostic
    const DETAILS: &str = "out of memory";

    /// What tsc wrote in a run whose report the action could not read
    const REPORT: &str = "tsc: The TypeScript Compiler - Version 7.0.2";

    // checktypescript[verify check.unreported]
    #[test]
    fn error_of_a_silent_run_holds_what_tsc_wrote() {
        let error = CheckTypeScriptError::from(ObserveTscError::MissingReport {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, CheckTypeScriptError::MissingReport { details } if details == DETAILS),
            "expected what tsc wrote, got {error:?}"
        );
    }

    // checktypescript[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_tsc_wrote() {
        let error = CheckTypeScriptError::from(ObserveTscError::UnreadableReport {
            report: REPORT.to_owned(),
            source: ReadReportError::UnreadableLine {
                line: REPORT.to_owned(),
            },
        });

        assert!(
            matches!(&error, CheckTypeScriptError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of tsc, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn check_type_script_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckTypeScriptError>();
    }
}
