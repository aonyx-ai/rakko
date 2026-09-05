use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;

/// An error that stops the reading of the Rust version of a workspace
///
/// Nothing here is a problem that a finding can report. A reading that
/// cannot ask cargo, or cannot read what cargo answered, does not know
/// whether the workspace declares a version at all, and a run that took the
/// silence for an absent declaration would skip a check that the project
/// asked for.
#[derive(Debug, Error)]
pub enum ReadRustVersionError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Cargo could not read the manifest of the workspace
    ///
    /// Cargo refused the manifest, or the workspace that the manifest names,
    /// and it wrote its diagnosis in the details.
    #[error("cargo cannot read the manifest {}: {details}", manifest.display())]
    UnreadableManifest {
        /// The manifest that cargo could not read
        manifest: PathBuf,

        /// What cargo wrote about the manifest
        details: String,
    },

    /// Cargo described a workspace in a shape that the crate does not
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
    fn read_rust_version_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ReadRustVersionError>();
    }
}
