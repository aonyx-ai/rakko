use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::tsc::report::ReadReportError;

/// An error that leaves a run of tsc without an answer
///
/// A rule of the language that the code breaks is no failure of a run: it
/// travels as a diagnostic, and the caller decides what it means. The variants
/// here describe a run that produced no answer about the project at all.
#[derive(Debug, Error)]
pub enum ObserveTscError {
    /// Tsc reported nothing and ended without success
    ///
    /// Tsc reports a diagnostic for everything that it finds, including for a
    /// configuration that it refuses and for a project that it finds no
    /// configuration of. A run that ended without success and said nothing
    /// therefore stopped for a reason that this crate cannot name, and the
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

    /// Tsc wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of tsc. A report that does
    /// not fit therefore points at a version that this crate does not know,
    /// and the diagnostics of such a run would be the ones that the reading
    /// happened to understand.
    #[error("tsc wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What tsc wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: ReadReportError,
    },
}

impl ObserveTscError {
    /// Returns the error of a run that ended without success and said nothing
    ///
    /// Tsc writes what it has to say on its standard output stream, so that is
    /// what the details carry. A run that wrote nothing there leaves its
    /// reason on the standard error stream, or says nothing at all.
    // checktypescript[impl check.unreported]
    pub(super) fn silent(stdout: &str, stderr: &str) -> Self {
        Self::MissingReport {
            details: diagnosis(stdout, stderr),
        }
    }
}

/// Returns what tsc wrote about a run that reported no diagnostic
///
/// The standard output stream answers, because that is where tsc writes. A run
/// that wrote nothing there says what happened on the standard error stream,
/// or says nothing at all, and the caller reports whichever of the two carries
/// text.
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

    // checktypescript[verify check.unreported]
    #[test]
    fn error_of_a_silent_run_holds_what_tsc_wrote() {
        let error = ObserveTscError::silent("out of memory", "");

        assert!(
            matches!(&error, ObserveTscError::MissingReport { details } if details == "out of memory"),
            "expected what tsc wrote, got {error:?}"
        );
    }

    // checktypescript[verify check.unreported]
    #[test]
    fn error_of_a_silent_run_that_wrote_nothing_holds_the_other_stream() {
        let error = ObserveTscError::silent("", "killed");

        assert!(
            matches!(&error, ObserveTscError::MissingReport { details } if details == "killed"),
            "expected what tsc wrote, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn observe_tsc_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveTscError>();
    }
}
