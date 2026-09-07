use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a question to tracey without an answer
///
/// A broken link between a specification and the code is no failure of a run:
/// it travels in the answer, and the caller decides what it means. The
/// variants here describe a question that produced no answer at all.
#[derive(Debug, Error)]
pub enum ObserveTraceyError {
    /// Tracey did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run tracey")]
    TraceyUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Tracey wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of tracey. A report that
    /// this crate cannot read therefore points at a version that it does not
    /// know, and the problems of such a run would be the ones that the reading
    /// happened to understand.
    #[error("tracey wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What tracey wrote in place of a report that the crate reads
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
    fn observe_tracey_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveTraceyError>();
    }
}
