use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of stiles without an answer
///
/// A diagnostic that a rule reported is no failure of a run: it travels in
/// the observation, and the caller decides what it means. The variants here
/// describe a run that produced no answer that the caller can trust.
#[derive(Debug, Error)]
pub enum ObserveStilesError {
    /// Stiles stopped before it had checked the project
    ///
    /// Stiles ends a run when it cannot read its configuration, when it cannot
    /// build or load a lint that the configuration names, and when it cannot
    /// read or analyze a file that it found. It checks nothing in the first
    /// two cases, and less than the project in the last, so an answer built on
    /// such a run would describe a part of the project as though it were the
    /// whole.
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

    /// Stiles wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of stiles. A report that
    /// this crate cannot read therefore points at a version that it does not
    /// know, and the findings of such a run would be the ones that the reading
    /// happened to understand.
    #[error("stiles wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What stiles wrote in place of a report that the crate reads
        report: String,

        /// The cause of the failure
        source: serde_json::Error,
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
    fn observe_stiles_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveStilesError>();
    }
}
