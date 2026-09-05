use std::path::PathBuf;

use rakko_cargo::DiscoverRootsError;
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::deny::CheckWorkspaceError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A shape that cargo-deny
/// recognized in the dependencies of a workspace travels as a finding in the
/// outcome of the run. The variants of this error describe a run whose answer
/// cannot be trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum CheckDependenciesError {
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

    /// A workspace of the project lies outside the project
    ///
    /// A finding names the workspace that it came from, relative to the
    /// project root, and a workspace outside the root has no such name. The
    /// discovery reports the workspaces below the project, so this points at
    /// a project that moved while the run was on.
    #[error("a workspace of the project lies outside it: {}", path.display())]
    ForeignWorkspace {
        /// The directory of the workspace root
        path: PathBuf,
    },

    /// Cargo-deny stopped before it had checked a workspace
    ///
    /// Cargo-deny reads the configuration of the project before it collects a
    /// single manifest, and it checks nothing when it cannot accept that
    /// configuration. It also stops at a workspace whose manifests it cannot
    /// read. Either way it wrote no summary of its checks, and an outcome
    /// built on such a report would describe a workspace that nothing
    /// examined.
    #[error("cargo-deny stopped before it had checked {}: {details}", root.display())]
    IncompleteCheck {
        /// The workspace root that the run was about
        root: PathBuf,

        /// What cargo-deny wrote about the run that it stopped
        details: String,
    },

    /// The workspaces of the project could not be discovered
    ///
    /// A run checks every workspace of the project, so a run that does not
    /// know them would check less than the project and report a green result
    /// for the rest.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo-deny wrote a record that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo-deny. A record
    /// that the action cannot read therefore points at a version that this
    /// crate does not know, and the findings of such a run would be the ones
    /// that the reading happened to understand.
    #[error("cargo-deny wrote a record that the action cannot read in {}: {record}", root.display())]
    UnreadableReport {
        /// The workspace root that the run was about
        root: PathBuf,

        /// The record that the reading stopped at
        record: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Mise reported no cargo for the project
    ///
    /// Cargo describes the workspaces that a run checks. The project pins no
    /// cargo, or nothing installed the pin yet, and the action installs
    /// nothing, so the run stops here.
    #[error("failed to resolve cargo")]
    UnresolvedCargo {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no cargo-deny for the project
    ///
    /// The project pins no cargo-deny, or nothing installed the pin yet. The
    /// action installs nothing, so the run stops here.
    #[error("failed to resolve cargo-deny")]
    UnresolvedDeny {
        /// The cause of the failure
        source: ResolveToolError,
    },
}

impl From<CheckWorkspaceError> for CheckDependenciesError {
    /// Turns the failure of a workspace check into the error of the action
    ///
    /// The machinery that runs cargo-deny names the same three conditions
    /// that the action reports, so the conversion renames them and adds
    /// nothing.
    // checkdependencies[impl check.configuration]
    // checkdependencies[impl check.incomplete]
    // checkdependencies[impl check.unreadable]
    fn from(error: CheckWorkspaceError) -> Self {
        match error {
            CheckWorkspaceError::DenyUnavailable { root, source } => {
                Self::DenyUnavailable { root, source }
            }
            CheckWorkspaceError::IncompleteCheck { root, details } => {
                Self::IncompleteCheck { root, details }
            }
            CheckWorkspaceError::UnreadableReport {
                root,
                record,
                source,
            } => Self::UnreadableReport {
                root,
                record,
                source,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that builds the
    // cause of a failure expects the reading to fail. A `# Panics` section on
    // every test would repeat that and give the reader no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What cargo-deny wrote about a configuration that it refused
    const DETAILS: &str = "failed to deserialize config from 'deny.toml'";

    /// A record that cargo-deny never wrote
    const RECORD: &str = "fatal: no check was performed";

    /// Returns the workspace root that the errors of a test are about
    fn root() -> PathBuf {
        PathBuf::from("/home/otter/project")
    }

    // checkdependencies[verify check.configuration]
    // checkdependencies[verify check.incomplete]
    #[test]
    fn error_of_a_run_that_stopped_holds_what_cargo_deny_wrote() {
        let error = CheckDependenciesError::from(CheckWorkspaceError::IncompleteCheck {
            root: root(),
            details: DETAILS.to_owned(),
        });

        assert!(
            matches!(&error, CheckDependenciesError::IncompleteCheck { details, .. } if details == DETAILS),
            "expected what cargo-deny wrote, got {error:?}"
        );
    }

    // checkdependencies[verify check.unreadable]
    #[test]
    fn error_of_an_unreadable_record_names_the_workspace_root() {
        let source = serde_json::from_str::<Vec<u8>>(RECORD)
            .expect_err("the test reads a record that is no record");
        let error = CheckDependenciesError::from(CheckWorkspaceError::UnreadableReport {
            root: root(),
            record: RECORD.to_owned(),
            source,
        });

        assert!(
            matches!(&error, CheckDependenciesError::UnreadableReport { root, .. } if root == &self::root()),
            "expected the workspace root, got {error:?}"
        );
    }

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn check_dependencies_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckDependenciesError>();
    }
}
