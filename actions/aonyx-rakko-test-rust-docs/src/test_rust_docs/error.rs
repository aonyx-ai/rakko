//! The error that stops a run of the action

use std::path::PathBuf;

use aonyx_rakko_cargo::{DiscoverRootsError, ReadReportError};
use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

use crate::report::ReadDoctestReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. An example that failed and a
/// diagnostic of a build that did not finish travel as findings in the
/// outcome of the run. The variants of this error describe a run whose
/// answer cannot be trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_test_rust_docs::TestRustDocsError;
/// use std::path::PathBuf;
///
/// let error = TestRustDocsError::StaleLockfile {
///     root: PathBuf::from("/home/otter/project"),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "the lockfile of the workspace in /home/otter/project needs an update"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TestRustDocsError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// The lockfile of a workspace needs an update
    ///
    /// The manifests of the workspace ask for something that the lockfile does
    /// not hold. An example is a new dependency or a new version of a member.
    /// The workspace may also have no lockfile. The action tests the versions
    /// that the lockfile holds and never writes it, so cargo refused the run.
    /// A build of the workspace writes the lockfile.
    #[error("the lockfile of the workspace in {} needs an update", root.display())]
    StaleLockfile {
        /// The workspace root whose lockfile needs an update
        root: PathBuf,
    },

    /// The workspaces of the project could not be discovered
    ///
    /// A run tests the examples of every workspace of the project. A run that
    /// does not know them all would leave the examples of the missing one
    /// unchecked behind a green result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo wrote a record about the build that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo. A line names a
    /// compiler message or the end of the build in a shape that the action
    /// does not know. That leaves the diagnostics of the build untrusted. An
    /// answer built on them would hide problems behind a green result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableDiagnostics {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// The test harness wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of the harness. A summary
    /// that does not count the examples leaves the run with a count that is
    /// too low. A count that is too low is a green result with examples
    /// missing from it.
    #[error("failed to read what the test harness reported in {}", root.display())]
    UnreadableReport {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadDoctestReportError,
    },

    /// A run of cargo ended in a way that the action does not recognize
    ///
    /// A run that ended without success reported no failed example and no
    /// diagnostic. It failed for a reason that the action cannot read, such as
    /// a build script that ended badly. An answer built on such a run would
    /// hide failures behind a green result.
    #[error("cargo ended a run in {} that the action does not recognize: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that cargo worked on
        root: PathBuf,

        /// What cargo wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no cargo for the project
    ///
    /// The project pins no stable Rust toolchain that the action accepts, or
    /// nothing installed the pin yet. The action installs nothing, so the run
    /// stops here.
    #[error("failed to resolve cargo")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveRequirementError,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Shows that `TestRustDocsError` is `Send` and `Sync`
    ///
    /// An action puts the error in the outcome of a run. That outcome holds an
    /// error that another thread can read. This test holds the error to the
    /// auto traits that make this possible. It does so because a field of a
    /// later version could take them away without a word from the compiler.
    #[test]
    fn test_rust_docs_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<TestRustDocsError>();
    }

    #[test]
    fn test_rust_docs_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<TestRustDocsError>();
    }
}
