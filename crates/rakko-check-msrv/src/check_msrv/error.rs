use std::path::PathBuf;

use rakko_cargo::{
    DiscoverRootsError, ReadReportError, ReadRustVersionError, ResolveToolchainError, RustVersion,
};
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A diagnostic of the compiler,
/// whether a warning or an error, travels as a finding in the outcome of the
/// run. The variants of this error describe a run whose answer cannot be
/// trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum CheckMsrvError {
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
    /// A run checks every workspace that declares a Rust version, and a run
    /// that does not know them all would hide every problem of the missing
    /// one behind a green result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// The declaration of a workspace could not be read
    ///
    /// The run asked cargo which Rust version the packages of the workspace
    /// declare, and cargo gave no answer that the action can use. A run that
    /// took the silence for an absent declaration would pass over a
    /// workspace that the project asked it to check.
    #[error("failed to read the rust-version that {} declares", root.display())]
    UnreadableDeclaration {
        /// The workspace root whose declaration was read
        root: PathBuf,

        /// The cause of the failure
        source: ReadRustVersionError,
    },

    /// Cargo wrote a record that the action cannot read
    ///
    /// The shape of a record belongs to a version of cargo. A line that
    /// names a compiler message or the end of the build in a shape that the
    /// action does not know leaves the report untrusted, and an answer built
    /// on it would hide problems behind a green result.
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
    /// ended without success and named no diagnostic, and a run that ended
    /// with success without saying that the build finished, both wrote
    /// something the action could not read, and an answer built on such a
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
    /// The project pins no Rust toolchain, or nothing installed the pin yet.
    /// The action installs nothing, so the run stops here.
    #[error("failed to resolve cargo")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no toolchain for the declared Rust version
    ///
    /// The project declares the version in a manifest, and mise pins no
    /// toolchain for it, or nothing installed the pin yet. The two places
    /// state the same version, so a disagreement between them arrives here.
    /// The action installs nothing, so the run stops.
    #[error("failed to resolve the toolchain for the declared rust-version {version}")]
    UnresolvedToolchain {
        /// The version that the workspace declares
        version: RustVersion,

        /// The cause of the failure
        source: ResolveToolchainError,
    },
}
