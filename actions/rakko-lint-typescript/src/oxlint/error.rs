//! The error that leaves a run without an answer

use bon::bon;
use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::oxlint::report::ReadReportError;

/// An error that leaves a run of oxlint without an answer
///
/// A diagnostic of oxlint is no failure of a run, even one about the project
/// itself: it travels in the observation, and the caller decides what it
/// means. The variants here describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub enum ObserveOxlintError {
    /// Oxlint wrote no report
    ///
    /// Oxlint reads its configuration before it collects a single file, and it
    /// writes no report when it cannot accept that configuration. A project
    /// whose configuration oxlint refuses therefore ends here, and so does any
    /// other run that stopped before it linted anything.
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

    /// Oxlint wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of oxlint. A report that
    /// does not fit therefore points at a version that this crate does not
    /// know, and the diagnostics of such a run would be the ones that the
    /// reading happened to understand.
    #[error("failed to read the report from oxlint: {report}")]
    UnreadableReport {
        /// What oxlint wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },
}

#[bon]
impl ObserveOxlintError {
    /// Returns the error of a run whose report the crate did not get
    ///
    /// A reading that found no report names a run that ended before it linted
    /// anything, and what oxlint wrote about that is its diagnosis rather than
    /// a report, so the error carries the text instead of the shape. Oxlint
    /// writes the diagnosis on its standard output stream, and a run that wrote
    /// nothing there leaves its reason on the standard error stream.
    #[builder]
    pub(super) fn of(
        source: ReadReportError,
        #[builder(into)] stdout: String,
        #[builder(into)] stderr: String,
    ) -> Self {
        match source {
            ReadReportError::Absent => Self::MissingReport {
                details: diagnosis().stdout(&stdout).stderr(&stderr).call(),
            },
            ReadReportError::Unreadable { .. } => Self::UnreadableReport {
                report: stdout,
                source,
            },
        }
    }
}

/// Returns what oxlint wrote about a run that produced no report
///
/// The standard output stream answers, because that is where oxlint writes its
/// diagnosis. A run that wrote nothing there says what happened on the standard
/// error stream, or says nothing at all, and the caller reports whichever of
/// the two carries text.
#[bon::builder]
fn diagnosis(stdout: &str, stderr: &str) -> String {
    let written = stdout.trim();

    if written.is_empty() {
        return stderr.trim().to_owned();
    }

    written.to_owned()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What oxlint writes when it refuses the configuration of a project
    const REJECTION: &str = "Failed to parse oxlint configuration file.";

    #[test]
    fn error_of_an_absent_report_holds_what_oxlint_wrote() {
        let error = ObserveOxlintError::of()
            .source(ReadReportError::Absent)
            .stdout(REJECTION)
            .stderr("")
            .call();

        assert!(
            matches!(&error, ObserveOxlintError::MissingReport { details } if details == REJECTION),
            "expected what oxlint wrote, got {error:?}"
        );
    }

    #[test]
    fn error_of_an_absent_report_of_a_silent_run_holds_the_other_stream() {
        let error = ObserveOxlintError::of()
            .source(ReadReportError::Absent)
            .stdout("")
            .stderr("killed")
            .call();

        assert!(
            matches!(&error, ObserveOxlintError::MissingReport { details } if details == "killed"),
            "expected what oxlint wrote, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn observe_oxlint_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveOxlintError>();
    }

    #[test]
    fn observe_oxlint_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ObserveOxlintError>();
    }
}
