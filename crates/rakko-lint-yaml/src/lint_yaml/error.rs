use std::path::PathBuf;

use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::yamllint::report::ReadReportError;
use crate::yamllint::{ListFilesError, ObserveYamllintError};

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A rule that a file broke travels
/// as a finding in the outcome of the run. The variants of this error
/// describe a run whose answer cannot be trusted, and such a run stops
/// instead of reporting one.
#[derive(Debug, Error)]
pub enum LintYamlError {
    /// Yamllint reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. A run names the root as the place
    /// to look, so this points at a report that the action misread.
    #[error("yamllint reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that yamllint reported
        path: PathBuf,
    },

    /// Yamllint stopped before it examined every file
    ///
    /// Yamllint ends a run at the first file that it cannot open, and the
    /// files that it had not reached stay unread. The problems that it had
    /// already reported describe a part of the project, and an outcome built
    /// on them would hide the rest of the project behind that part.
    #[error("yamllint stopped before it examined every file: {details}")]
    IncompleteExamination {
        /// What yamllint wrote about the file that stopped it
        details: String,
    },

    /// Yamllint refused the configuration of the project
    ///
    /// Yamllint reads its configuration before it collects a single file, and
    /// it lints nothing when it cannot accept that configuration. The project
    /// asked for rules that never applied, so the run stops here.
    #[error("yamllint refused the configuration of the project: {details}")]
    RejectedConfiguration {
        /// What yamllint wrote about the configuration
        details: String,
    },

    /// Yamllint wrote a report that the action cannot read
    ///
    /// The shape of a line belongs to a version of yamllint. A report that
    /// holds a line which names no problem therefore points at a version that
    /// this crate does not know, and the findings of such a run would be the
    /// lines that the reading happened to understand.
    #[error("yamllint wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What yamllint wrote in place of a report that the action reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Mise reported no yamllint for the project
    ///
    /// The project pins no yamllint, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve yamllint")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
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

impl From<ListFilesError> for LintYamlError {
    /// Turns the failure of a listing into the error of the action
    ///
    /// The machinery that lists the files names the same two conditions that
    /// the action reports, so the conversion renames them and adds nothing.
    // lintyaml[impl check.configuration]
    fn from(error: ListFilesError) -> Self {
        match error {
            ListFilesError::RejectedConfiguration { details } => {
                Self::RejectedConfiguration { details }
            }
            ListFilesError::YamllintUnavailable { source } => Self::YamllintUnavailable { source },
        }
    }
}

impl From<ObserveYamllintError> for LintYamlError {
    /// Turns the failure of a yamllint run into the error of the action
    ///
    /// The machinery that runs yamllint names the same two conditions that
    /// the action reports, so the conversion renames them and adds nothing.
    // lintyaml[impl check.unreadable]
    fn from(error: ObserveYamllintError) -> Self {
        match error {
            ObserveYamllintError::UnreadableReport { report, source } => {
                Self::UnreadableReport { report, source }
            }
            ObserveYamllintError::YamllintUnavailable { source } => {
                Self::YamllintUnavailable { source }
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

    /// What yamllint wrote about a configuration that it refused
    const DETAILS: &str = "invalid config: no such rule: \"no-such-rule\"";

    /// What yamllint wrote in a run whose report the crate could not read
    const REPORT: &str = "Traceback (most recent call last):";

    // lintyaml[verify check.configuration]
    #[test]
    fn error_of_a_refused_configuration_holds_what_yamllint_wrote() {
        let error = LintYamlError::from(ListFilesError::RejectedConfiguration {
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, LintYamlError::RejectedConfiguration { details } if details == DETAILS),
            "expected what yamllint wrote, got {error:?}"
        );
    }

    // lintyaml[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_report_holds_what_yamllint_wrote() {
        let error = LintYamlError::from(ObserveYamllintError::UnreadableReport {
            report: REPORT.to_owned(),
            source: ReadReportError::UnreadableLine {
                line: REPORT.to_owned(),
            },
        });

        assert!(
            matches!(&error, LintYamlError::UnreadableReport { report, .. } if report == REPORT),
            "expected the report of yamllint, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn lint_yaml_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<LintYamlError>();
    }
}
