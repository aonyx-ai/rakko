use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that leaves a workspace unchecked
///
/// A shape that cargo-deny recognized in the dependencies of a workspace is
/// no failure of a run: it travels as a problem, and the caller decides what
/// it means. The variants here describe a run that produced no answer about
/// the workspace at all.
#[derive(Debug, Error)]
pub enum CheckWorkspaceError {
    /// Cargo-deny did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the workspace was checked.
    #[error("failed to run cargo-deny in {}", root.display())]
    DenyUnavailable {
        /// The workspace root that the run was about
        root: PathBuf,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// Cargo-deny stopped before it had checked the workspace
    ///
    /// Cargo-deny ends every run that reached its checks with a summary of
    /// them. A report without that summary belongs to a run that stopped
    /// earlier, which it does when it cannot read the configuration of the
    /// project or the manifests of the workspace. An answer built on such a
    /// report would describe a workspace that nothing examined.
    #[error("cargo-deny stopped before it had checked {}: {details}", root.display())]
    IncompleteCheck {
        /// The workspace root that the run was about
        root: PathBuf,

        /// What cargo-deny wrote about the run that it stopped
        details: String,
    },

    /// Cargo-deny wrote a record that the crate cannot read
    ///
    /// The shape of a record belongs to a version of cargo-deny. A record
    /// that this crate cannot read therefore points at a version that it does
    /// not know, and the problems of such a run would be the ones that the
    /// reading happened to understand.
    #[error("cargo-deny wrote a record that the action cannot read in {}: {record}", root.display())]
    UnreadableReport {
        /// The workspace root that the run was about
        root: PathBuf,

        /// The record that the reading stopped at
        record: String,

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
    fn check_workspace_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckWorkspaceError>();
    }
}
