use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run of markdownlint without an answer
///
/// A rule that a file broke is no failure of a run: it travels in the
/// observation, and the caller decides what it means. The variants here
/// describe a run that produced no answer at all.
#[derive(Debug, Error)]
pub enum ObserveMarkdownlintError {
    /// Markdownlint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run markdownlint")]
    MarkdownlintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Markdownlint wrote a report that the crate cannot read
    ///
    /// The shape of the report belongs to a version of markdownlint. A run
    /// that could not open a file also ends this way, because markdownlint
    /// then writes the failure of its runtime in place of the report, so an
    /// answer built on such a report would leave an unknown part of the
    /// project unread.
    #[error("markdownlint wrote a report that the action cannot read: {report}")]
    UnreadableReport {
        /// What markdownlint wrote in place of a report
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
    fn observe_markdownlint_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ObserveMarkdownlintError>();
    }
}
