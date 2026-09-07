use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a run without the comparison that it asked for
///
/// A run that finds no pull request to compare is no failure: it answers that
/// there is none, and the caller compares the checkout itself. The variants
/// here describe a comparison that could not be built at all.
#[derive(Debug, Error)]
pub enum PrepareComparisonError {
    /// Git did not run
    ///
    /// Git is infrastructure of a machine, and a project that has none cannot
    /// be compared with the branch that it builds on.
    #[error("failed to run git")]
    GitUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Git refused to build the copy
    ///
    /// The repository resolved the revisions of the comparison and then
    /// refused a step of it, such as a worktree that it would not add.
    #[error("git refused to copy the repository: {details}")]
    RefusedCopy {
        /// What git wrote about the step that it refused
        details: String,
    },

    /// The copy has nowhere to live
    ///
    /// The comparison needs a directory of the temporary directory of the
    /// system, and the system gave none.
    #[error("failed to create a directory for the copy of the repository")]
    UnavailableDirectory {
        /// The cause of the failure
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read.
    #[test]
    fn prepare_comparison_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<PrepareComparisonError>();
    }
}
