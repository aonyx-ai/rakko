use std::path::PathBuf;

use rakko_cargo::{DiscoverRootsError, ReadReportError};
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::report::ReadDoctestReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. An example that failed and a
/// diagnostic of a build that did not finish travel as findings in the
/// outcome of the run. The variants of this error describe a run whose
/// answer cannot be trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
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

    /// The workspaces of the project could not be discovered
    ///
    /// A run tests the examples of every workspace of the project, and a run
    /// that does not know them all would leave the examples of the missing
    /// one unchecked behind a green result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo wrote a record about the build that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo. A line that
    /// names a compiler message or the end of the build in a shape that the
    /// action does not know leaves the diagnostics of the build untrusted,
    /// and an answer built on them would hide problems behind a green
    /// result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableDiagnostics {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// The test harness wrote a report that the action cannot read
    ///
    /// The shape of the report belongs to a version of the harness. A
    /// summary that does not count the examples leaves the run with a count
    /// that is too low, and a count that is too low is a green result with
    /// examples missing from it.
    #[error("failed to read what the test harness reported in {}", root.display())]
    UnreadableReport {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadDoctestReportError,
    },

    /// A run of cargo ended in a way that the action does not recognize
    ///
    /// A run that ended without success and reported no failed example and
    /// no diagnostic failed for a reason that the action cannot read, such
    /// as a build script that ended badly, and an answer built on such a run
    /// would hide failures behind a green result.
    #[error("cargo ended a run in {} that the action does not recognize: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that cargo worked on
        root: PathBuf,

        /// What cargo wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no cargo for the project
    ///
    /// The project pins no Rust toolchain, or nothing installed the pin yet.
    /// The action installs nothing, so the run stops here.
    #[error("failed to resolve cargo")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
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
    fn test_rust_docs_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<TestRustDocsError>();
    }
}
