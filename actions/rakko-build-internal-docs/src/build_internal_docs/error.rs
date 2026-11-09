//! The error that stops a run of the action

use std::path::PathBuf;

use rakko_cargo::{DiscoverRootsError, ReadReportError};
use rakko_tool::{ResolveRequirementError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the documentation. A diagnostic of rustdoc,
/// whether a warning or an error of the compiler, travels as a finding in
/// the outcome of the run. The variants of this error describe a run whose
/// answer cannot be trusted, and such a run stops instead of reporting one.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use rakko_build_internal_docs::BuildInternalDocsError;
///
/// let error = BuildInternalDocsError::StaleLockfile {
///     root: PathBuf::from("/home/otter/project"),
/// };
/// assert_eq!(
///     error.to_string(),
///     "the lockfile of the workspace in /home/otter/project needs an update"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BuildInternalDocsError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was documented.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// The lockfile of a workspace needs an update
    ///
    /// The manifests of the workspace ask for something that the lockfile does
    /// not hold. Examples include a new dependency or a new version of a member.
    /// Alternatively, the workspace has no lockfile. The action documents the versions that the
    /// lockfile holds and never writes it, so cargo refused the run. A build of
    /// the workspace writes the lockfile.
    #[error("the lockfile of the workspace in {} needs an update", root.display())]
    StaleLockfile {
        /// The workspace root whose lockfile needs an update
        root: PathBuf,
    },

    /// The workspaces of the project could not be discovered
    ///
    /// A run documents every workspace of the project. A run that does not know
    /// them all would hide every problem of the missing one behind a green
    /// result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo wrote a record that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo. A line that names
    /// a compiler message or the end of the build in a shape the action does
    /// not know leaves the report untrusted. An answer built on it would hide
    /// problems behind a green result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableReport {
        /// The workspace root that cargo worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Cargo wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of cargo. A run that
    /// ended without success and named no diagnostic wrote something the action
    /// could not read. A run that ended with success without saying that the
    /// build finished also wrote such a report. An answer built on such a
    /// report would hide problems behind a green result.
    #[error("cargo wrote a report in {} that the action does not recognize: {stderr}", root.display())]
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
    use super::*;

    #[test]
    fn build_internal_docs_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<BuildInternalDocsError>();
    }
}
