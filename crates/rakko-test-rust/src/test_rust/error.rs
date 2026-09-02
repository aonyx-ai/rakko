use std::path::PathBuf;

use rakko_cargo::{DiscoverRootsError, ReadReportError};
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use super::report::ReadNextestReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A test that failed and a
/// diagnostic of a build that did not finish travel as findings in the
/// outcome of the run. The variants of this error describe a run whose
/// answer cannot be trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum TestRustError {
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
    /// A run tests every workspace of the project, and a run that does not
    /// know them all would hide every failure of the missing one behind a
    /// green result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo wrote a record that the action cannot read
    ///
    /// Nextest forwards the JSON of cargo on the same stream, and the shape
    /// of a record belongs to a version of cargo. A line that names a
    /// compiler message or the end of the build in a shape that the action
    /// does not know leaves the diagnostics of the build untrusted, and an
    /// answer built on them would hide problems behind a green result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableDiagnostics {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Nextest wrote a record that the action cannot read
    ///
    /// The shape of a record belongs to a version of nextest. A line about a
    /// test or a binary of tests in a shape that the action does not know
    /// leaves the report untrusted, and an answer built on it would hide
    /// failures behind a green result.
    #[error("failed to read what nextest reported in {}", root.display())]
    UnreadableReport {
        /// The workspace root that nextest worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadNextestReportError,
    },

    /// Nextest wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of nextest. A run that
    /// ended without success and reported no failure, no diagnostic, and no
    /// absence of tests wrote something the action could not read, and an
    /// answer built on such a report would hide failures behind a green
    /// result.
    #[error("nextest wrote a report in {} that the action does not recognize: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that nextest worked on
        root: PathBuf,

        /// What nextest wrote to its standard error stream
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
