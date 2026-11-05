//! The error that leaves a caller without the files of a project

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a caller without the files that yamllint examines
///
/// A project without a YAML file is no failure of the listing: the listing is
/// empty then, and the caller decides what that means. The variants here
/// describe a listing that produced no answer at all.
///
/// # Examples
///
/// ```
/// use rakko_lint_yaml::ListFilesError;
///
/// let error = ListFilesError::RejectedConfiguration {
///     details: "invalid config option".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "yamllint refused the configuration of the project: invalid config option"
/// );
/// ```
#[derive(Debug, Error)]
pub enum ListFilesError {
    /// Yamllint refused the configuration of the project
    ///
    /// Yamllint reads its configuration before it collects a single file. It
    /// stops when it cannot accept that configuration. A run of the action
    /// stops with it, because yamllint lints nothing in this state. A report
    /// that never arrived says nothing about the project.
    #[error("yamllint refused the configuration of the project: {details}")]
    RejectedConfiguration {
        /// What yamllint wrote about the configuration
        details: String,
    },

    /// Yamllint did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was collected.
    #[error("failed to run yamllint")]
    YamllintUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Shows that `ListFilesError` is `Send` and `Sync`
    ///
    /// An action puts the error in the outcome of a run. That outcome holds an
    /// error that another thread can read. This test holds the error to the
    /// auto traits that make this possible. This matters because a field of a
    /// later version could take them away without a word from the compiler.
    #[test]
    fn list_files_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ListFilesError>();
    }

    #[test]
    fn list_files_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ListFilesError>();
    }
}
