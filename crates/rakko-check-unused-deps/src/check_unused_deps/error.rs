use std::path::PathBuf;

use rakko_cargo::{DiscoverRootsError, ReadReportError, ResolveToolchainError};
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

use crate::report::ReadUdepsReportError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A dependency that no target
/// loads, and a diagnostic of a build that did not finish, both travel as
/// findings in the outcome of the run. The variants of this error describe a
/// run whose answer cannot be trusted, and such a run stops instead of
/// reporting one.
#[derive(Debug, Error)]
pub enum CheckUnusedDepsError {
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
    /// A run examines every workspace, and a run that does not know them all
    /// would hide every unused dependency of the missing one behind a green
    /// result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Cargo wrote a record that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo. A line that
    /// names a compiler message or the end of the build in a shape that the
    /// action does not know leaves the report untrusted, and an answer built
    /// on it would hide problems behind a green result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableCargoReport {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Cargo-udeps wrote its report in a shape that the action cannot read
    ///
    /// The shape belongs to a version of cargo-udeps, and an answer built on
    /// a report that the action could not read would hide every unused
    /// dependency of the workspace behind a green result.
    #[error("failed to read what cargo-udeps reported in {}", root.display())]
    UnreadableUdepsReport {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadUdepsReportError,
    },

    /// The run wrote no report that the action can answer from
    ///
    /// A run that ended without success and named neither an unused
    /// dependency nor a diagnostic, and a run that ended with success and
    /// wrote no report of cargo-udeps, both leave the action without an
    /// answer, and a result built on such a run would hide every unused
    /// dependency behind a green result.
    #[error("the run in {} wrote no report that the action recognizes: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// What the tools wrote to the standard error stream
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

    /// Mise reported no toolchain for the nightly channel of the project
    ///
    /// Cargo-udeps reads the crates that a target loaded from a record that
    /// only an unstable option of the compiler writes, so the build needs
    /// the nightly channel. The action installs nothing, so a project that
    /// pins no nightly stops the run.
    #[error("failed to resolve the nightly toolchain")]
    UnresolvedToolchain {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}
