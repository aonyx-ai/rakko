use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of zizmor without an answer
///
/// A pattern that zizmor recognized in a workflow is no failure of a run: it
/// travels in the observation, and the caller decides what it means. The
/// variants here describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub enum ObserveZizmorError {
    /// Zizmor stopped before it had audited every file
    ///
    /// Zizmor ends a run when it cannot read the configuration of the project,
    /// and, because the run asks it to, when it cannot read a file that it
    /// collected. It audits nothing in the first case, and less than the
    /// project in the second, so an answer built on such a run would describe
    /// a part of the project as though it were the whole.
    #[error("zizmor stopped before it had audited the project: {details}")]
    IncompleteAudit {
        /// What zizmor wrote about the run that it stopped
        details: String,
    },

    /// Zizmor wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of zizmor. A report that
    /// this crate cannot read therefore points at a version that it does not
    /// know, and the findings of such a run would be the ones that the reading
    /// happened to understand.
    #[error("zizmor wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What zizmor wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
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
    fn observe_zizmor_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveZizmorError>();
    }
}
