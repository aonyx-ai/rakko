use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that stops the discovery of the workspace roots of a project
///
/// Nothing here is a problem that a finding can report. A discovery that
/// cannot read a directory or a manifest cannot say which roots the project
/// holds, and an answer with a root missing would hide every problem of that
/// root behind a green run. The discovery therefore stops, and the action
/// that asked for it reports the error.
#[derive(Debug, Error)]
pub enum DiscoverRootsError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// A manifest of the project belongs to a workspace outside the project
    ///
    /// An outer workspace that lists the manifest as a member is the usual
    /// case. Cargo names that outer directory as the root, and a job there
    /// would work on files outside the project, so the discovery stops
    /// instead of naming a root that the project does not contain.
    #[error(
        "the manifest {} belongs to a workspace outside the project, at {}",
        manifest.display(),
        workspace.display()
    )]
    ForeignWorkspace {
        /// The manifest of the project that cargo assigned to the workspace
        manifest: PathBuf,

        /// The directory of the workspace, which lies outside the project
        workspace: PathBuf,
    },

    /// A directory of the project could not be read
    ///
    /// The discovery walks every directory of the project that can hold a
    /// manifest, and a directory that it cannot read can hold a root that it
    /// would never find.
    #[error("failed to read the directory {}", directory.display())]
    UnreadableDirectory {
        /// The directory that could not be read
        directory: PathBuf,

        /// The cause of the failure
        source: std::io::Error,
    },

    /// Cargo could not read a manifest of the project
    ///
    /// Cargo refused the manifest, or the workspace that the manifest names.
    /// A package that sits under a workspace which does not list it is the
    /// usual case, and cargo names it in the details.
    #[error("cargo cannot read the manifest {}: {details}", manifest.display())]
    UnreadableManifest {
        /// The manifest that cargo could not read
        manifest: PathBuf,

        /// What cargo wrote about the manifest
        details: String,
    },

    /// Cargo described a manifest in a shape that the crate does not
    /// recognize
    ///
    /// The shape of the description belongs to a version of cargo, and a
    /// description that the crate cannot read leaves it without an answer.
    #[error("failed to read what cargo reported about the manifest {}", manifest.display())]
    UnrecognizedMetadata {
        /// The manifest that cargo described
        manifest: PathBuf,

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
    fn discover_roots_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<DiscoverRootsError>();
    }
}
